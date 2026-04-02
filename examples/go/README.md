# Go example app

App pequeña pensada para probar herramientas de navegación.

Casos útiles:

- `trace_flow` desde handlers HTTP
- `trace_callers` sobre services o repositorios
- `find_symbol` sobre interfaces, métodos y funciones

Punto de entrada sugerido para pruebas:

- `cmd/api/main.go`
- `internal/http/handlers/user_handler.go`
- `internal/service/user_service.go`
- `internal/repository/memory_user_repository.go`
