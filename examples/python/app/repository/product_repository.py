from typing import List, Optional
from ..models.product import Product


class ProductRepository:
    def __init__(self):
        self._db: List[Product] = [
            Product(
                id=1, name="Keyboard", price=120.0, description="Mechanical Keyboard"
            ),
            Product(id=2, name="Mouse", price=80.0, description="Wireless Mouse"),
            Product(id=3, name="Monitor", price=300.0, description="4K Monitor"),
        ]

    def find_by_id(self, product_id: int) -> Optional[Product]:
        """Direct DB simulation query."""
        for p in self._db:
            if p.id == product_id:
                return p
        return None

    def list_all(self) -> List[Product]:
        """Fetch all from DB."""
        return self._db

    def save(self, product: Product) -> None:
        """Persist to DB."""
        self._db.append(product)
