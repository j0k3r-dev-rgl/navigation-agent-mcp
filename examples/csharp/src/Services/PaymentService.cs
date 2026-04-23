using NavigationExample.Domain;

namespace NavigationExample.Services;

public class StripePaymentProcessor : IPaymentProcessor
{
    public async Task<bool> ProcessPaymentAsync(PaymentRequest paymentRequest)
    {
        if (!IsValid(paymentRequest))
        {
            return false;
        }

        var authorized = await AuthorizeAsync(paymentRequest);
        if (!authorized)
        {
            return false;
        }

        return await CaptureAsync(paymentRequest);
    }

    private static bool IsValid(PaymentRequest paymentRequest) =>
        paymentRequest.Amount > 0 && !string.IsNullOrWhiteSpace(paymentRequest.CustomerName);

    private static Task<bool> AuthorizeAsync(PaymentRequest paymentRequest) =>
        Task.FromResult(paymentRequest.Amount <= 500m);

    private static Task<bool> CaptureAsync(PaymentRequest paymentRequest) =>
        Task.FromResult(paymentRequest.OrderId != Guid.Empty);
}

public class EmailNotificationService : INotificationService
{
    public Task SendNotificationAsync(string email, string message)
    {
        return Task.CompletedTask;
    }
}
