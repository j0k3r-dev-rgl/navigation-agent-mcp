# Python Examples (FastAPI-style)

Este directorio contiene una estructura Python para probar el motor de navegación con un caso de uso de "Store API".

## Estructura del proyecto

```text
app/
├── main.py              # Entrypoint (FastAPI)
├── api/
│   └── endpoints.py     # Routes & decorators
├── models/
│   └── product.py       # Classes & Dataclasses
└── services/
    └── inventory.py     # Business logic
```

## Casos de Prueba

- **Symbol Discovery**: Localizar `Product`, `InventoryService`, `list_products`.
- **Flow Tracing**: Seguir `get_product` (API) -> `get_product_by_id` (Service) -> `Product` (Model).
- **Callers Tracing**: Encontrar quién usa `get_product_by_id`.
- **Endpoint Listing**: Detectar los decoradores `@router.get` y `@app.get`.
