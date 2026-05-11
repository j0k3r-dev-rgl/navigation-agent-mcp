# C# example app

App pequeña de .NET pensada para probar navegación semántica en C# con un flujo de órdenes controlado.

## Estructura

```text
src/
├── Program.cs                    # Punto de entrada para pruebas manuales
├── ExampleApp.csproj             # Proyecto .NET mínimo
├── Domain/
│   ├── Models.cs                 # records, enum y clase de soporte
│   └── Interfaces.cs             # IOrderRepository, IPaymentProcessor, INotificationService
├── Infrastructure/
│   └── OrderRepository.cs        # Repositorio en memoria con datos semilla
└── Services/
    ├── OrderWorkflowService.cs   # Flujo principal de procesamiento
    ├── OrderReportService.cs     # Lecturas y reporting
    └── PaymentService.cs         # Procesamiento de pago y notificaciones
```

## Casos de prueba implementados

### 1. `find_symbol`

Símbolos interesantes para validar el soporte C#:

- **Records**: `OrderLine`, `Order`, `ProcessOrderRequest`, `PaymentRequest`, `PendingReviewSummary`
- **Interfaces**: `IOrderRepository`, `IPaymentProcessor`, `INotificationService`
- **Clases**: `OrderWorkflowService`, `OrderReportService`, `OrderRepository`, `StripePaymentProcessor`, `EmailNotificationService`
- **Métodos**: `ProcessOrderAsync`, `BuildPendingReviewReportAsync`, `ProcessPaymentAsync`, `ListPendingReviewAsync`, `GetOrderDetailsAsync`

### 2. `trace_flow`

**Ejemplo 1: `OrderWorkflowService.ProcessOrderAsync`**

```text
ProcessOrderAsync
  -> LoadDraftOrderAsync -> repository.GetByIdAsync
  -> EnsureProcessable
  -> ApplyDiscount -> NormalizeDiscount
  -> BuildPaymentRequest
  -> paymentProcessor.ProcessPaymentAsync
       -> IsValid
       -> AuthorizeAsync
       -> CaptureAsync
  -> PersistPaidOrderAsync | PersistPendingReviewAsync
       -> repository.SaveAsync
  -> NotifyPaidOrderAsync | NotifyPendingReviewAsync
       -> notificationService.SendNotificationAsync
```

**Ejemplo 2: `OrderReportService.BuildPendingReviewReportAsync`**

```text
BuildPendingReviewReportAsync
  -> repository.ListPendingReviewAsync
  -> Select(... PendingReviewSummary ...)
  -> ToList
```

### 3. `trace_callers`

Métodos con múltiples callers útiles para análisis de impacto:

- `IOrderRepository.GetByIdAsync`
  - `OrderWorkflowService.LoadDraftOrderAsync`
  - `OrderWorkflowService.GetOrderDetailsAsync`
  - `OrderReportService.GetOrderSnapshotAsync`

- `IOrderRepository.ListByCustomerAsync`
  - `OrderWorkflowService.GetCustomerHistoryAsync`
  - `OrderReportService.CalculateCustomerLifetimeValueAsync`

- `IOrderRepository.ListPendingReviewAsync`
  - `OrderReportService.BuildPendingReviewReportAsync`

- `IOrderRepository.SaveAsync`
  - `OrderWorkflowService.PersistPaidOrderAsync`
  - `OrderWorkflowService.PersistPendingReviewAsync`

## Punto de entrada sugerido

- `src/Program.cs`
- `src/Services/OrderWorkflowService.cs`
- `src/Services/OrderReportService.cs`
- `src/Infrastructure/OrderRepository.cs`

## Propósito

Este ejemplo está diseñado para:

1. Validar búsqueda de símbolos C# sobre records, interfaces, clases y métodos.
2. Probar `trace_flow` con helpers privados y cruces entre servicios e infraestructura.
3. Probar `trace_callers` sobre métodos reutilizados por más de un servicio.

## Verificación

Se pueden validar el funcionamiento de `trace_callers` y `trace_flow` ejecutando los scripts de runtime:
```bash
node test-runtime/test-csharp-trace-callers.mjs
node test-runtime/test-csharp-trace-flow.mjs
```
