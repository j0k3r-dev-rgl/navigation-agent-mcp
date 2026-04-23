using NavigationExample.Domain;

namespace NavigationExample.Services;

public class OrderWorkflowService
{
    private readonly IOrderRepository _repository;
    private readonly IPaymentProcessor _paymentProcessor;
    private readonly INotificationService _notificationService;

    public OrderWorkflowService(
        IOrderRepository repository,
        IPaymentProcessor paymentProcessor,
        INotificationService notificationService)
    {
        _repository = repository;
        _paymentProcessor = paymentProcessor;
        _notificationService = notificationService;
    }

    public async Task<bool> ProcessOrderAsync(ProcessOrderRequest request)
    {
        var draftOrder = await LoadDraftOrderAsync(request.OrderId);
        EnsureProcessable(draftOrder);

        var pricedOrder = ApplyDiscount(draftOrder, request.DiscountPercentage);
        var paymentRequest = BuildPaymentRequest(pricedOrder);
        var paymentAccepted = await _paymentProcessor.ProcessPaymentAsync(paymentRequest);

        if (!paymentAccepted)
        {
            await PersistPendingReviewAsync(pricedOrder);
            await NotifyPendingReviewAsync(request.CustomerEmail, pricedOrder);
            return false;
        }

        await PersistPaidOrderAsync(pricedOrder);
        await NotifyPaidOrderAsync(request.CustomerEmail, pricedOrder);
        return true;
    }

    public Task<Order?> GetOrderDetailsAsync(Guid orderId) => _repository.GetByIdAsync(orderId);

    public Task<IReadOnlyList<Order>> GetCustomerHistoryAsync(Guid customerId) =>
        _repository.ListByCustomerAsync(customerId);

    private async Task<Order> LoadDraftOrderAsync(Guid orderId)
    {
        var order = await _repository.GetByIdAsync(orderId);
        return order ?? throw new InvalidOperationException($"Order {orderId} was not found");
    }

    private static void EnsureProcessable(Order order)
    {
        if (order.Status is OrderStatus.Cancelled or OrderStatus.Paid)
        {
            throw new InvalidOperationException($"Order {order.Id} cannot be processed from state {order.Status}");
        }
    }

    private static Order ApplyDiscount(Order order, decimal discountPercentage)
    {
        var normalizedDiscount = NormalizeDiscount(discountPercentage);
        var discountedTotal = order.TotalAmount * (1 - normalizedDiscount);
        return order with { TotalAmount = decimal.Round(discountedTotal, 2), Status = OrderStatus.Approved };
    }

    private static decimal NormalizeDiscount(decimal discountPercentage)
    {
        if (discountPercentage <= 0)
        {
            return 0m;
        }

        return Math.Min(discountPercentage / 100m, 0.40m);
    }

    private static PaymentRequest BuildPaymentRequest(Order order) =>
        new(order.Id, order.TotalAmount, order.CustomerName);

    private Task PersistPendingReviewAsync(Order order) =>
        _repository.SaveAsync(order with { Status = OrderStatus.PendingReview });

    private Task PersistPaidOrderAsync(Order order) =>
        _repository.SaveAsync(order with { Status = OrderStatus.Paid });

    private Task NotifyPendingReviewAsync(string email, Order order) =>
        _notificationService.SendNotificationAsync(email, $"Order {order.Id} requires review");

    private Task NotifyPaidOrderAsync(string email, Order order) =>
        _notificationService.SendNotificationAsync(email, $"Order {order.Id} was paid successfully");
}
