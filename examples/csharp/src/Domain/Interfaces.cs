namespace NavigationExample.Domain;

public interface IOrderRepository
{
    Task<Order?> GetByIdAsync(Guid id);
    Task<IReadOnlyList<Order>> ListByCustomerAsync(Guid customerId);
    Task<IReadOnlyList<Order>> ListPendingReviewAsync();
    Task SaveAsync(Order order);
}

public interface IPaymentProcessor
{
    Task<bool> ProcessPaymentAsync(PaymentRequest paymentRequest);
}

public interface INotificationService
{
    Task SendNotificationAsync(string email, string message);
}
