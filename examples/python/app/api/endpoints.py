from fastapi import APIRouter, HTTPException, Depends
from typing import List
from ..models.product import Product
from ..models.user import User
from ..services.inventory import InventoryService
from ..services.order_service import OrderService
from ..services.user_service import UserService

router = APIRouter()
inventory_service = InventoryService()
order_service = OrderService()
user_service = UserService()


@router.get("/products", response_model=List[Product])
def list_products():
    """List all available products."""
    return inventory_service.get_all_products()


@router.get("/products/{product_id}", response_model=Product)
def get_product(product_id: int):
    """Retrieve a single product by ID."""
    product = inventory_service.get_product_by_id(product_id)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")
    return product


@router.post("/products", status_code=201)
def create_product(product: Product):
    """Add a new product."""
    inventory_service.add_product(product)
    return {"message": "Product created successfully"}


@router.post("/orders/{product_id}", response_model=Product)
def create_order(product_id: int, user_email: str, quantity: int = 1):
    """Test deep order flow trace with multiple branches."""
    product = order_service.process_order(product_id, quantity, user_email)
    if not product:
        raise HTTPException(
            status_code=404, detail="Order processing failed: check product or payment"
        )
    return product


@router.get("/users/{username}", response_model=User)
def get_user(username: str):
    """Test deep user flow trace."""
    user = user_service.get_user_profile(username)
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    return user


@router.post("/users", response_model=User)
def register_user(username: str, email: str):
    """Register user with audit."""
    return user_service.register_user(username, email)
