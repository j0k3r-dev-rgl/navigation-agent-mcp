from datetime import datetime


class AuditService:
    def log_action(self, action: str, details: str) -> None:
        """Simple audit logger to test deep trace flow."""
        timestamp = datetime.now().isoformat()
        print(f"[{timestamp}] AUDIT: {action} - {details}")

    def log_error(self, error_msg: str) -> None:
        """Audit for errors."""
        timestamp = datetime.now().isoformat()
        print(f"[{timestamp}] AUDIT ERROR: {error_msg}")
