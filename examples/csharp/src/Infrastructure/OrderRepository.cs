using NavigationExample.Domain;

namespace NavigationExample.Infrastructure;

public class OrderRepository : IOrderRepository
{
    private readonly Dictionary<Guid, Order> _orders = SeedOrders();

    public Task<Order?> GetByIdAsync(Guid id)
    {
        _orders.TryGetValue(id, out var order);
        return Task.FromResult(order);
    }

    public Task<IReadOnlyList<Order>> ListByCustomerAsync(Guid customerId)
    {
        var orders = _orders.Values
            .Where(order => order.CustomerId == customerId)
            .OrderBy(order => order.CustomerName)
            .ToList();

        return Task.FromResult<IReadOnlyList<Order>>(orders);
    }

    public Task<IReadOnlyList<Order>> ListPendingReviewAsync()
    {
        var orders = _orders.Values
            .Where(order => order.Status == OrderStatus.PendingReview)
            .OrderByDescending(order => order.TotalAmount)
            .ToList();

        return Task.FromResult<IReadOnlyList<Order>>(orders);
    }

    public Task SaveAsync(Order order)
    {
        _orders[order.Id] = order;
        return Task.CompletedTask;
    }

    private static Dictionary<Guid, Order> SeedOrders()
    {
        var customerId = Guid.Parse("11111111-1111-1111-1111-111111111111");
        var pendingReviewId = Guid.Parse("22222222-2222-2222-2222-222222222222");
        var paidOrderId = Guid.Parse("33333333-3333-3333-3333-333333333333");

        return new Dictionary<Guid, Order>
        {
            [pendingReviewId] = new(
                pendingReviewId,
                customerId,
                "Alice",
                220m,
                OrderStatus.PendingReview,
                new List<OrderLine>
                {
                    new("sku-keyboard", 1, 120m),
                    new("sku-mouse", 2, 50m),
                }),
            [paidOrderId] = new(
                paidOrderId,
                customerId,
                "Alice",
                80m,
                OrderStatus.Paid,
                new List<OrderLine>
                {
                    new("sku-headphones", 1, 80m),
                }),
        };
    }
}
