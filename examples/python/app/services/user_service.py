from typing import Optional
from .audit import AuditService
from ..repository.user_repository import UserRepository
from ..models.user import User


class UserService:
    def __init__(self):
        self.audit_service = AuditService()
        self.repo = UserRepository()

    def get_user_profile(self, username: str) -> Optional[User]:
        """Deep call chain: Service -> Audit -> Repo -> Model."""
        self.audit_service.log_action("get_profile", f"Fetching profile for {username}")

        user = self.repo.find_by_username(username)
        if user:
            self.audit_service.log_action(
                "profile_found", f"Found {user.username} with role {user.role}"
            )
            return user

        self.audit_service.log_error(f"User {username} not found")
        return None

    def register_user(self, username: str, email: str) -> User:
        """Create new user with audit."""
        self.audit_service.log_action("register_user", f"New registration: {username}")

        user = User(id=3, username=username, email=email)
        self.repo.save(user)

        self.audit_service.log_action("registration_complete", username)
        return user
