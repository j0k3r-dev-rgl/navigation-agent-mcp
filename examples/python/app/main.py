from fastapi import FastAPI
from .api.endpoints import router as product_router

app = FastAPI(title="Store API", version="1.0.0")

app.include_router(product_router, prefix="/api/v1")


@app.get("/")
def health_check():
    """Service health check endpoint."""
    return {"status": "ok", "service": "Store API"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8000)
