class PaymentService:
    def authorize_payment(self, amount: float) -> bool:
        """Simulate payment authorization."""
        return self._validate_funds(amount)

    def _validate_funds(self, amount: float) -> bool:
        """Private-ish method to test internal calls."""
        return amount < 1000.0

    def capture_funds(self, amount: float) -> None:
        """Capture the funds after authorization."""
        print(f"Funds captured: {amount}")
