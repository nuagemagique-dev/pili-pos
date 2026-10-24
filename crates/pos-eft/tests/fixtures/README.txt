Verbatim examples from public documentation, used by the parsing tests of
pos-eft. JSON cannot hold comments, so the sources are listed here.

worldline_*.json
    Examples of the Worldline Terminal API v2 OpenAPI (nexo Retailer 5.1,
    version 5.1-WL2.1.1), converted from YAML to JSON without changing a value:
    https://wjx44tdpp7.execute-api.eu-west-3.amazonaws.com/OpenAPI/terminal-api-openapi-spec/openapi-v2.yaml
    (rendered at https://docs.terminal.worldline-solutions.com/api-reference)
    SimplePaymentRequest, SimplePaymentResponse, PaymentRequestWithTipResponse,
    ResponseReasonAbortedResponse, ResponseReasonRefusalResponse,
    SimplePaymentErrorResponse, ReferencedPaymentStatusRequest/Response,
    AbortPaymentRequest, MessageRejectionResponse.

ccv_terminal_create_request.json, ccv_terminal_create_response.json
    The terminal method of the CCV Payment API, request and response examples:
    https://developer.myccv.eu/documentation/payment_api/online_payments/payment_methods/terminal/

ccv_terminal_success.json, ccv_terminal_failed_cancelled.json
    NOT verbatim: CCV publishes no finished terminal transaction. Built from the
    OpenAPI schema (https://api.psp.ccv.eu/api/v1/openapi): field names and enums
    are CCV's, the values (cardCircuit, receipt lines) are made up.

opi_card_service_response_italian_profile.xml, opi_device_request_italian_profile.xml
    Sections 4.2 and 6.2.1 of the OPI interface of the Italian profile
    (Diebold Nixdorf / Aevi), the only public finished OPI samples:
    http://www.sebeto.com/intranet/ftpscambio/BuonoChiaro/Aevi%20Pay-OPI-Interface%201.16.pdf
    (the DeviceRequest lines are shortened and its target changed from Printer to
    PrinterReceipt so that it is a customer receipt).

opi_card_request_de_minimal.xml
    The minimal CardPayment request that works on a real OPI-DE terminal:
    https://gist.github.com/t-oster/a1bed2bb82d566bbdb90706d521a8f83
