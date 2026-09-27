//! HubRise API types shared between the HubRise client (`pos-hubrise`)
//! and the mock server (`pos-hubrise-mock`).
//!
//! Reference: the HubRise documentation snapshots under `docs/hubrise/` at
//! the repository root. Any schema change on the HubRise side must be
//! reflected here AND in `HUBRISE_DOCS_REVISION` of `pos-hubrise-mock`.

pub mod callback;
pub mod catalog;
pub mod common;
pub mod custom_fields;
pub mod error;
pub mod money;
pub mod oauth;
pub mod order;

pub use callback::{
    CallbackEvent, CallbackRegistration, CatalogUpdateEvent, EventResource, OrderCreateEvent,
    OrderUpdateEvent,
};
pub use catalog::{
    Catalog, CatalogData, CatalogSummary, Category, Charge, Deal, Discount, OptionList, Product,
    Sku, SkuOption, Variant,
};
pub use common::{Language, Money, PriceOverride, Restrictions, ServiceType, TaxRate};
pub use custom_fields::{
    CategoryDescriptionPayload, CategoryTranslations, ProductTranslation, ProductTranslations,
    SkuCustomFields, VatCategory,
};
pub use error::{ErrorResponse, ErrorType};
pub use oauth::{AuthorizeParams, OAuthTokenResponse, RevokeRequest};
pub use order::{
    Customer, Order, OrderCharge, OrderDealEntry, OrderDiscount, OrderItem, OrderOption,
    OrderPayment, OrderStatus,
};
