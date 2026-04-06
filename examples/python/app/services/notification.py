class NotificationService:
    def send_order_confirmation(self, user_email: str, order_id: str) -> None:
        """Entry point for order notifications."""
        self._format_template(order_id)
        self._dispatch_email(user_email)

    def _format_template(self, order_id: str) -> str:
        """Format the email content."""
        return f"Order {order_id} confirmed."

    def _dispatch_email(self, email: str) -> None:
        """Simulate email sending."""
        print(f"Email sent to {email}")
