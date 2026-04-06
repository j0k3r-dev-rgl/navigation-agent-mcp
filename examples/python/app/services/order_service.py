from typing import Optional
from .inventory import InventoryService
from .audit import AuditService
from .payment import PaymentService
from .notification import NotificationService
from ..repository.product_repository import ProductRepository
from ..models.product import Product


class OrderService:
    def __init__(self):
        self.inventory_service = InventoryService()
        self.audit_service = AuditService()
        self.payment_service = PaymentService()
        self.notification_service = NotificationService()
        self.repo = ProductRepository()

    def process_order(
        self, product_id: int, quantity: int, user_email: str
    ) -> Optional[Product]:
        """Main entry point for order processing with multiple branches."""
        self.audit_service.log_action("order_start", f"Processing {product_id}")

        # Branch 1: Validation
        product = self._validate_and_get_product(product_id)
        if not product:
            return None

        # Branch 2: Payment
        total_price = product.price * quantity
        if not self._handle_payment(total_price):
            return None

        # Branch 3: Fulfillment
        self._finalize_order(product, user_email)

        return product

    def _validate_and_get_product(self, product_id: int) -> Optional[Product]:
        """Sub-branch for validation logic."""
        product = self.inventory_service.get_product_by_id(product_id)
        if product:
            # Check deep repository call
            self.repo.find_by_id(product_id)
        return product

    def _handle_payment(self, amount: float) -> bool:
        """Sub-branch for payment flow."""
        if self.payment_service.authorize_payment(amount):
            self.payment_service.capture_funds(amount)
            return True
        self.audit_service.log_error("payment_failed")
        return False

    def _finalize_order(self, product: Product, email: str) -> None:
        """Sub-branch for confirmation and auditing."""
        product.display_name()
        self.notification_service.send_order_confirmation(email, str(product.id))
        self.audit_service.log_action("order_complete", f"Finished {product.id}")
