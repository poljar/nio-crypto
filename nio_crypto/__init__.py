import json

from typing import Dict, Any, Set, List
from collections import defaultdict
from .nio_crypto import OlmMachine as Machine, Request


class OlmMachine:
    def __init__(self, user_id: str, device_id: str):
        self.inner = Machine(user_id, device_id)

    def update_tracked_users(self, users: Set[str]):
        return self.inner.update_tracked_users(users)

    def should_share_group_session(self, room_id: str) -> bool:
        return self.inner.should_share_group_session(room_id)

    def mark_request_as_sent(
        self, request_id: str, request_type: str, response: Dict[Any, Any]
    ):
        return self.inner.mark_request_as_sent(
            request_id, request_type, json.dumps(response)
        )

    def outgoing_requests(self) -> List[Request]:
        return self.inner.outgoing_requests()

    def share_group_session(
        self, room_id: str, users: List[str],
    ) -> List[Dict[str, Dict[str, str]]]:
        messages = self.inner.share_group_session(room_id, users)
        json_messages = []

        for message in messages:
            json_message = defaultdict(dict)

            for user, content_map in message.items():
                for device_id, content in content_map.items():
                    json_message[user][device_id] = json.loads(content)

            json_messages.append(json_message)

        return json_messages

    def get_missing_sessions(self, users: List[str]) -> Dict[str, Dict[str, str]]:
        return self.inner.get_missing_sessions(users)

    def encrypt(self, room_id: str, content: Dict[str, str]):
        return json.loads(self.inner.encrypt(room_id, json.dumps(content)))

    def receive_sync_response(self, response_body: Dict[Any, Any]):
        return self.inner.receive_sync_response(json.dumps(response_body))
