using NavigationExample.Domain;

namespace NavigationExample.Services;

public class OrderReportService
{
    private readonly IOrderRepository _repository;

    public OrderReportService(IOrderRepository repository)
    {
        _repository = repository;
    }

    public async Task<IReadOnlyList<PendingReviewSummary>> BuildPendingReviewReportAsync()
    {
        var pendingOrders = await _repository.ListPendingReviewAsync();
        return pendingOrders
            .Select(order => new PendingReviewSummary(order.Id, order.CustomerName, order.TotalAmount))
            .ToList();
    }

    public async Task<decimal> CalculateCustomerLifetimeValueAsync(Guid customerId)
    {
        var orders = await _repository.ListByCustomerAsync(customerId);
        return orders.Sum(order => order.TotalAmount);
    }

    public Task<Order?> GetOrderSnapshotAsync(Guid orderId) => _repository.GetByIdAsync(orderId);
}
