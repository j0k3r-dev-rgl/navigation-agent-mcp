from typing import List, Optional
from ..models.user import User


class UserRepository:
    def __init__(self):
        self._users: List[User] = [
            User(id=1, username="j0k3r", email="j0k3r@example.com", role="admin"),
            User(id=2, username="guest", email="guest@example.com"),
        ]

    def find_by_username(self, username: str) -> Optional[User]:
        """Find a user by username."""
        for user in self._users:
            if user.username == username:
                return user
        return None

    def save(self, user: User) -> None:
        """Persist a new user."""
        self._users.append(user)
