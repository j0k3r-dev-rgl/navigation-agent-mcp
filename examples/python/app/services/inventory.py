from typing import List, Optional
from .product import Product


class InventoryService:
    def __init__(self):
        self.products: List[Product] = [
            Product(
                id=1, name="Keyboard", price=120.0, description="Mechanical Keyboard"
            ),
            Product(id=2, name="Mouse", price=80.0, description="Wireless Mouse"),
        ]

    def get_all_products(self) -> List[Product]:
        """Fetch all products in inventory."""
        return self.products

    def get_product_by_id(self, product_id: int) -> Optional[Product]:
        """Find a product by its ID."""
        for product in self.products:
            if product.id == product_id:
                return product
        return None

    def add_product(self, product: Product) -> None:
        """Add a new product to inventory."""
        self.products.append(product)
