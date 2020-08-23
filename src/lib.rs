use std::{
    collections::{BTreeMap, HashSet},
    convert::TryFrom,
};

use futures::executor::block_on;

use pyo3::prelude::*;

use http::Response;
use serde_json::json;

use matrix_sdk_common::{
    api::r0::{
        keys::{
            claim_keys::Response as KeysClaimResponse, get_keys::Response as KeysQueryResponse,
            upload_keys::Response as KeysUploadResponse,
        },
        sync::sync_events::Response as SyncResponse,
    },
    events::room::message::MessageEventContent,
    identifiers::{RoomId, UserId},
    uuid::Uuid,
};
use matrix_sdk_crypto::{
    EncryptionSettings, IncomingResponse, OlmMachine as Machine, OutgoingRequests,
};

#[pyclass]
struct OlmMachine {
    inner: Machine,
}

#[pyclass]
struct Request {
    #[pyo3(get)]
    pub request_id: String,
    #[pyo3(get)]
    pub request_type: String,
    #[pyo3(get)]
    pub body: String,
}

enum OwnedResponse {
    KeysClaim(KeysClaimResponse),
    KeysUpload(KeysUploadResponse),
    KeysQuery(KeysQueryResponse),
}

impl From<KeysClaimResponse> for OwnedResponse {
    fn from(response: KeysClaimResponse) -> Self {
        OwnedResponse::KeysClaim(response)
    }
}

impl From<KeysQueryResponse> for OwnedResponse {
    fn from(response: KeysQueryResponse) -> Self {
        OwnedResponse::KeysQuery(response)
    }
}

impl From<KeysUploadResponse> for OwnedResponse {
    fn from(response: KeysUploadResponse) -> Self {
        OwnedResponse::KeysUpload(response)
    }
}

impl<'a> Into<IncomingResponse<'a>> for &'a OwnedResponse {
    fn into(self) -> IncomingResponse<'a> {
        match self {
            OwnedResponse::KeysClaim(r) => IncomingResponse::KeysClaim(r),
            OwnedResponse::KeysQuery(r) => IncomingResponse::KeysQuery(r),
            OwnedResponse::KeysUpload(r) => IncomingResponse::KeysUpload(r),
        }
    }
}

fn response_from_string(body: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(200)
        .body(body.as_bytes().to_vec())
        .expect("Can't create HTTP response")
}

#[pymethods]
impl OlmMachine {
    #[new]
    fn new(user_id: &str, device_id: &str) -> Self {
        Self {
            inner: Machine::new(
                &UserId::try_from(user_id).expect("Invalid user_id"),
                device_id.into(),
            ),
        }
    }

    fn should_share_group_session(&self, room_id: &str) -> bool {
        let room_id = RoomId::try_from(room_id).expect(&format!("Invalid room id {}", room_id));
        self.inner.should_share_group_session(&room_id)
    }

    fn mark_request_as_sent(&self, request_id: &str, request_type: &str, response: &str) {
        let request_id = Uuid::parse_str(request_id).unwrap();
        let response = response_from_string(response);

        let response: OwnedResponse = match request_type {
            "keys_upload" => KeysUploadResponse::try_from(response).map(Into::into),
            "keys_query" => KeysQueryResponse::try_from(response).map(Into::into),
            "keys_claim" => KeysClaimResponse::try_from(response).map(Into::into),
            _ => panic!("Unknown response"),
        }
        .expect("Can't convert json string to response");

        block_on(self.inner.mark_requests_as_sent(&request_id, &response))
            .expect("Error while handling response");
    }

    fn outgoing_requests(&self) -> Vec<Request> {
        block_on(self.inner.outgoing_requests())
            .iter()
            .map(|r| {
                let (request_type, body) = match r.request() {
                    OutgoingRequests::KeysQuery(r) => (
                        "keys_query",
                        serde_json::to_string(&json!({"device_keys": r.device_keys})).unwrap(),
                    ),
                    OutgoingRequests::KeysUpload(r) => (
                        "keys_upload",
                        serde_json::to_string(&json!({
                            "device_keys": r.device_keys,
                            "one_time_keys": r.one_time_keys,
                        }))
                        .unwrap(),
                    ),
                    _ => panic!("To-device requests aren't yet supported"),
                };

                Request {
                    request_id: r.request_id().to_string(),
                    request_type: request_type.to_owned(),
                    body,
                }
            })
            .collect()
    }

    fn get_missing_sessions(
        &self,
        mut users: Vec<String>,
    ) -> Option<(String, BTreeMap<String, BTreeMap<String, String>>)> {
        let users: Vec<UserId> = users
            .drain(..)
            .filter_map(|u| UserId::try_from(u).ok())
            .collect();

        let (request_id, missing) =
            block_on(self.inner.get_missing_sessions(users.iter())).unwrap()?;

        Some((
            request_id.to_string(),
            missing
                .one_time_keys
                .iter()
                .map(|(u, m)| {
                    (
                        u.to_string(),
                        m.iter()
                            .map(|(d, k)| (d.to_string(), k.to_string()))
                            .collect(),
                    )
                })
                .collect(),
        ))
    }

    fn share_group_session(
        &self,
        room_id: &str,
        mut users: Vec<String>,
    ) -> Vec<BTreeMap<String, BTreeMap<String, String>>> {
        let room_id = RoomId::try_from(room_id).expect("Invalid room id");
        let users: Vec<UserId> = users
            .drain(..)
            .filter_map(|u| UserId::try_from(u).ok())
            .collect();

        let requests = block_on(self.inner.share_group_session(
            &room_id,
            users.iter(),
            EncryptionSettings::default(),
        ))
        .expect("Can't share group session");

        requests
            .iter()
            .map(|r| {
                r.messages
                    .iter()
                    .map(|(u, m)| {
                        (
                            u.to_string(),
                            m.iter()
                                .map(|(d, v)| (d.to_string(), v.get().to_owned()))
                                .collect(),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    fn update_tracked_users(&self, users: HashSet<String>) {
        let users: HashSet<UserId> = users
            .iter()
            .filter_map(|u| UserId::try_from(u.as_ref()).ok())
            .collect();
        block_on(self.inner.update_tracked_users(&users))
    }

    fn encrypt(&self, room_id: &str, content: &str) -> String {
        let room_id = RoomId::try_from(room_id).expect(&format!("Invalid room id {}", room_id));
        let content: MessageEventContent = serde_json::from_str(content).unwrap();
        serde_json::to_string(
            &block_on(self.inner.encrypt(&room_id, content)).expect("Can't encrypt event content"),
        )
        .unwrap()
    }

    fn receive_sync_response(&self, json_response: &str) -> PyResult<()> {
        let response = response_from_string(json_response);
        let mut response = SyncResponse::try_from(response).expect("Can't parse response");
        block_on(self.inner.receive_sync_response(&mut response));
        Ok(())
    }
}

#[pymodule]
fn nio_crypto(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<OlmMachine>()?;
    m.add_class::<Request>()?;
    Ok(())
}
