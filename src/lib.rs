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
    identifiers::{RoomId, UserId},
};
use matrix_sdk_crypto::{EncryptionSettings, OlmMachine as Machine};

#[pyclass]
struct OlmMachine {
    inner: Machine,
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

    fn should_upload_keys(&self) -> bool {
        block_on(self.inner.should_upload_keys())
    }

    fn should_share_group_session(&self, room_id: &str) -> bool {
        let room_id = RoomId::try_from(room_id).expect(&format!("Invalid room id {}", room_id));
        self.inner.should_share_group_session(&room_id)
    }

    fn should_query_keys(&self) -> bool {
        block_on(self.inner.should_query_keys())
    }

    fn users_for_key_query(&self) -> HashSet<String> {
        let users = block_on(self.inner.users_for_key_query());
        users.iter().map(|u| u.to_string()).collect()
    }

    fn get_missing_sessions(
        &self,
        mut users: Vec<String>,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        let users: Vec<UserId> = users
            .drain(..)
            .filter_map(|u| UserId::try_from(u).ok())
            .collect();
        let missing = block_on(self.inner.get_missing_sessions(users.iter())).unwrap();
        missing
            .iter()
            .map(|(u, m)| {
                (
                    u.to_string(),
                    m.iter()
                        .map(|(d, k)| (d.to_string(), k.to_string()))
                        .collect(),
                )
            })
            .collect()
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
        println!("HELLLOFROM RUST {:?}", users);
        let users: HashSet<UserId> = users
            .iter()
            .filter_map(|u| UserId::try_from(u.as_ref()).ok())
            .collect();
        block_on(self.inner.update_tracked_users(&users))
    }

    fn keys_for_upload(&self) -> String {
        let request = block_on(self.inner.keys_for_upload()).unwrap();

        serde_json::to_string(&json!({
            "device_keys": request.device_keys,
            "one_time_keys": request.one_time_keys,
        }))
        .unwrap()
    }

    fn receive_keys_upload_response(&self, json_response: &str) -> PyResult<()> {
        let response = response_from_string(json_response);
        let response = KeysUploadResponse::try_from(response).expect("Can't parse response");
        block_on(self.inner.receive_keys_upload_response(&response)).unwrap();
        Ok(())
    }

    fn receive_keys_query_response(&self, json_response: &str) -> PyResult<()> {
        let response = response_from_string(json_response);
        let response = KeysQueryResponse::try_from(response).expect("Can't parse response");
        block_on(self.inner.receive_keys_query_response(&response)).unwrap();
        Ok(())
    }

    fn receive_keys_claim_response(&self, json_response: &str) -> PyResult<()> {
        let response = response_from_string(json_response);
        let response = KeysClaimResponse::try_from(response).expect("Can't parse response");
        block_on(self.inner.receive_keys_claim_response(&response)).unwrap();
        Ok(())
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
    Ok(())
}
