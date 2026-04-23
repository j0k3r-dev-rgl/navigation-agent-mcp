namespace NavigationExample.Domain;

public record OrderLine(string Sku, int Quantity, decimal UnitPrice)
{
    public decimal GetSubtotal() => Quantity * UnitPrice;
}

public record Order(
    Guid Id,
    Guid CustomerId,
    string CustomerName,
    decimal TotalAmount,
    OrderStatus Status,
    IReadOnlyList<OrderLine> Lines);

public record ProcessOrderRequest(Guid OrderId, string CustomerEmail, decimal DiscountPercentage);

public record PaymentRequest(Guid OrderId, decimal Amount, string CustomerName);

public record PendingReviewSummary(Guid OrderId, string CustomerName, decimal TotalAmount);

public enum OrderStatus
{
    Draft,
    PendingReview,
    Approved,
    Paid,
    Cancelled
}

public class CustomerProfile
{
    public Guid UserId { get; init; }
    public List<Order> OrderHistory { get; init; } = new();
}
