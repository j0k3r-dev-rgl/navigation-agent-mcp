using NavigationExample.Domain;
using NavigationExample.Infrastructure;
using NavigationExample.Services;

var repository = new OrderRepository();
var paymentProcessor = new StripePaymentProcessor();
var notificationService = new EmailNotificationService();
var workflowService = new OrderWorkflowService(repository, paymentProcessor, notificationService);
var reportService = new OrderReportService(repository);

var orderId = Guid.Parse("22222222-2222-2222-2222-222222222222");
var customerId = Guid.Parse("11111111-1111-1111-1111-111111111111");

var result = await workflowService.ProcessOrderAsync(
    new ProcessOrderRequest(orderId, "alice@example.com", 10m));

var pendingReview = await reportService.BuildPendingReviewReportAsync();
var orderSnapshot = await workflowService.GetOrderDetailsAsync(orderId);
var lifetimeValue = await reportService.CalculateCustomerLifetimeValueAsync(customerId);

Console.WriteLine($"Order result: {result}");
Console.WriteLine($"Pending review count: {pendingReview.Count}");
Console.WriteLine($"Order snapshot status: {orderSnapshot?.Status}");
Console.WriteLine($"Customer lifetime value: {lifetimeValue}");
