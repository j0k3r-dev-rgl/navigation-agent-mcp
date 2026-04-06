from dataclasses import dataclass
from typing import Optional


@dataclass
class Product:
    id: int
    name: str
    price: float
    description: Optional[str] = None

    def display_name(self) -> str:
        return f"{self.name} (${self.price})"
