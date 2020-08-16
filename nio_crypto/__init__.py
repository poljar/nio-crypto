import json

from typing import Dict, Any, Set
from .nio_crypto import OlmMachine as Machine


class OlmMachine:
    def __init__(self, user_id: str, device_id: str):
        self.inner = Machine(user_id, device_id)

    @property
    def should_upload_keys(self) -> bool:
        return self.inner.should_upload_keys()

    def update_tracked_users(self, users: Set[str]):
        return self.inner.update_tracked_users(users)

    @property
    def should_share_group_session(self, room_id: str) -> bool:
        return self.inner.should_share_group_session(room_id)

    @property
    def should_query_keys(self) -> bool:
        return self.inner.should_query_keys()

    def keys_for_upload(self) -> Dict[Any, Any]:
        return json.loads(self.inner.keys_for_upload())

    def get_missing_sessions(self, users: List[str]) -> Dict[str, Dict[str, str]]:
        return self.inner.get_missing_sessions(users)

    @property
    def users_for_key_query(self) -> Set[str]:
        return self.inner.users_for_key_query()

    def receive_keys_upload_response(self, response_body: Dict[Any, Any]):
        return self.inner.receive_keys_upload_response(
            json.dumps(response_body)
        )

    def receive_keys_claim_response(self, response_body: Dict[Any, Any]):
        return self.inner.receive_keys_claim_response(
            json.dumps(response_body)
        )

    def receive_keys_query_response(self, response_body: Dict[Any, Any]):
        return self.inner.receive_keys_query_response(
            json.dumps(response_body)
        )

    def receive_sync_response(self, response_body: Dict[Any, Any]):
        return self.inner.receive_sync_response(
            json.dumps(response_body)
        )
