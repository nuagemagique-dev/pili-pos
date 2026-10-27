//! Test data: a seed catalog shaped like a sushi restaurant's, an
//! order builder, and the order templates the control API injects.
//!
//! Everything here is plain `HubRise` wire JSON, so Rust tests, the
//! control API and a live run against the real `HubRise` share it.

use pos_hubrise_types::CatalogData;
use pos_hubrise_types::money::{format_cents, parse_cents, parse_quantity};
use serde_json::{Value, json};

/// Tax rates of food: 12% eaten in, 6% taken away or delivered.
pub const FOOD: (&str, &str, &str) = ("12.0", "6.0", "6.0");
/// Tax rates of soft drinks: 21% eaten in, 6% taken away or delivered.
pub const SOFT_DRINK: (&str, &str, &str) = ("21.0", "6.0", "6.0");
/// Tax rates of alcohol: 21% everywhere.
pub const ALCOHOL: (&str, &str, &str) = ("21.0", "21.0", "21.0");

/// A catalog built category by category and product by product.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct CatalogBuilder {
    categories: Vec<Value>,
    products: Vec<Value>,
    option_lists: Vec<Value>,
    charges: Vec<Value>,
}

/// One SKU of a product: `(ref, name, price in cents, option lists)`.
pub type SkuSpec<'a> = (&'a str, Option<&'a str>, i64, &'a [&'a str]);

impl CatalogBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn category(mut self, ref_: &str, name: &str) -> Self {
        self.categories.push(json!({ "ref": ref_, "name": name }));
        self
    }

    /// A product with its tax rates `(eat_in, collection, delivery)`.
    pub fn product(
        mut self,
        ref_: &str,
        category: &str,
        name: &str,
        tax: (&str, &str, &str),
        skus: &[SkuSpec<'_>],
    ) -> Self {
        let skus: Vec<Value> = skus
            .iter()
            .map(|(sku_ref, sku_name, cents, lists)| {
                json!({
                    "ref": sku_ref,
                    "name": sku_name,
                    "price": format_cents(*cents),
                    "option_list_refs": lists,
                })
            })
            .collect();
        self.products.push(json!({
            "ref": ref_,
            "category_ref": category,
            "name": name,
            "tax_rate": { "eat_in": tax.0, "collection": tax.1, "delivery": tax.2 },
            "skus": skus,
        }));
        self
    }

    /// Edit the last product added (tags, custom fields, ...).
    pub fn with_last_product(mut self, edit: impl FnOnce(&mut Value)) -> Self {
        if let Some(p) = self.products.last_mut() {
            edit(p);
        }
        self
    }

    /// An option list: `(ref, name, price in cents)` per option.
    pub fn option_list(
        mut self,
        ref_: &str,
        name: &str,
        min: u32,
        max: Option<u32>,
        options: &[(&str, &str, i64)],
    ) -> Self {
        let options: Vec<Value> = options
            .iter()
            .map(|(o_ref, o_name, cents)| {
                json!({ "ref": o_ref, "name": o_name, "price": format_cents(*cents) })
            })
            .collect();
        self.option_lists.push(json!({
            "ref": ref_,
            "name": name,
            "min_selections": min,
            "max_selections": max,
            "options": options,
        }));
        self
    }

    pub fn charge(mut self, ref_: &str, name: &str, kind: &str, cents: Option<i64>) -> Self {
        self.charges.push(json!({
            "ref": ref_,
            "name": name,
            "type": kind,
            "price": cents.map(format_cents),
        }));
        self
    }

    pub fn build_json(self) -> Value {
        json!({
            "categories": self.categories,
            "products": self.products,
            "option_lists": self.option_lists,
            "charges": self.charges,
            "deals": [],
            "discounts": [],
            "variants": [],
        })
    }

    pub fn build(self) -> CatalogData {
        serde_json::from_value(self.build_json()).expect("builder makes valid catalogs")
    }
}

/// The seed catalog: a small sushi menu with food and drinks, two
/// sizes of one product, a shared option list and a delivery charge.
pub fn seed_catalog() -> CatalogData {
    seed_catalog_builder().build()
}

pub fn seed_catalog_builder() -> CatalogBuilder {
    CatalogBuilder::new()
        .category("MAKI", "Makis")
        .category("SOUPES", "Soupes")
        .category("BOISSONS", "Boissons")
        .product(
            "MAKI-SAUMON",
            "MAKI",
            "Maki saumon",
            FOOD,
            &[
                ("MAKI-SAUMON-6", Some("6 pièces"), 550, &["SAUCE"]),
                ("MAKI-SAUMON-12", Some("12 pièces"), 1000, &["SAUCE"]),
            ],
        )
        .product(
            "CALIFORNIA",
            "MAKI",
            "California roll",
            FOOD,
            &[("CALIFORNIA-8", None, 890, &["SAUCE", "EXTRA"])],
        )
        .with_last_product(|p| {
            p["skus"][0]["custom_fields"] = json!({
                "translations": { "zh": { "name": "加州卷" } }
            });
        })
        .product(
            "MISO",
            "SOUPES",
            "Soupe miso",
            FOOD,
            &[("MISO-1", None, 350, &[])],
        )
        .product(
            "COCA",
            "BOISSONS",
            "Coca-Cola 33cl",
            SOFT_DRINK,
            &[("COCA-33", None, 300, &[])],
        )
        .product(
            "ASAHI",
            "BOISSONS",
            "Bière Asahi 33cl",
            ALCOHOL,
            &[("ASAHI-33", None, 450, &[])],
        )
        .option_list(
            "SAUCE",
            "Sauce",
            0,
            Some(1),
            &[
                ("SAUCE-SOJA", "Soja", 0),
                ("SAUCE-SUCREE", "Soja sucrée", 50),
            ],
        )
        .option_list(
            "EXTRA",
            "Suppléments",
            0,
            None,
            &[
                ("EXTRA-AVOCAT", "Avocat", 100),
                ("EXTRA-CHEESE", "Cheese", 80),
            ],
        )
        .charge("DELIVERY", "Livraison", "delivery", Some(250))
}

/// A `HubRise` order body built line by line. Totals follow `HubRise`:
/// an item costs `(price + options) x quantity`, charges add,
/// discounts subtract.
#[derive(Debug, Clone)]
#[must_use]
pub struct OrderBuilder {
    order: Value,
}

impl OrderBuilder {
    /// An order from `channel` (`"Deliveroo"`, `"Website"`, ...).
    pub fn new(channel: &str, service_type: &str) -> Self {
        Self {
            order: json!({
                "status": "new",
                "channel": channel,
                "service_type": service_type,
                "asap": true,
                "items": [],
                "charges": [],
                "discounts": [],
                "payments": [],
            }),
        }
    }

    pub fn field(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.order[key] = value.into();
        self
    }

    pub fn item(mut self, sku_ref: &str, name: &str, cents: i64, quantity: i64) -> Self {
        self.items_mut().push(json!({
            "product_name": name,
            "sku_ref": sku_ref,
            "price": format_cents(cents),
            "quantity": quantity.to_string(),
            "options": [],
        }));
        self
    }

    /// Set the last item's tax rate (`"6.0"`).
    pub fn tax_rate(mut self, rate: &str) -> Self {
        if let Some(item) = self.items_mut().last_mut() {
            item["tax_rate"] = rate.into();
        }
        self
    }

    /// An option on the last item.
    pub fn option(mut self, list_name: &str, name: &str, ref_: &str, cents: i64) -> Self {
        if let Some(item) = self.items_mut().last_mut() {
            item["options"]
                .as_array_mut()
                .expect("an order item always has an options array")
                .push(json!({
                    "option_list_name": list_name,
                    "name": name,
                    "ref": ref_,
                    "price": format_cents(cents),
                    "quantity": 1,
                }));
        }
        self
    }

    pub fn charge(mut self, ref_: &str, name: &str, cents: i64) -> Self {
        self.order["charges"]
            .as_array_mut()
            .expect("an order always has a charges array")
            .push(json!({
                "ref": ref_,
                "name": name,
                "price": format_cents(cents),
            }));
        self
    }

    pub fn discount(mut self, ref_: &str, name: &str, cents: i64) -> Self {
        self.order["discounts"]
            .as_array_mut()
            .expect("an order always has a discounts array")
            .push(json!({ "ref": ref_, "name": name, "price_off": format_cents(cents) }));
        self
    }

    pub fn payment(mut self, ref_: &str, name: &str, cents: i64) -> Self {
        self.order["payments"]
            .as_array_mut()
            .expect("an order always has a payments array")
            .push(json!({ "ref": ref_, "name": name, "amount": format_cents(cents) }));
        self
    }

    /// One payment of the whole total, as a platform pays.
    pub fn paid_by(self, ref_: &str, name: &str) -> Self {
        let total = order_total_cents(&self.order);
        self.payment(ref_, name, total)
    }

    pub fn customer(mut self, customer: Value) -> Self {
        self.order["customer"] = customer;
        self
    }

    pub fn build(self) -> Value {
        self.order
    }

    fn items_mut(&mut self) -> &mut Vec<Value> {
        self.order["items"]
            .as_array_mut()
            .expect("an order always has an items array")
    }
}

fn cents_of(v: &Value) -> i64 {
    v.as_str().and_then(|s| parse_cents(s).ok()).unwrap_or(0)
}

fn not_deleted(v: &Value) -> bool {
    !v.get("deleted").and_then(Value::as_bool).unwrap_or(false)
}

/// The total `HubRise` computes for an order body.
pub fn order_total_cents(order: &Value) -> i64 {
    let list = |key: &str| {
        order
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let items: i64 = list("items")
        .iter()
        .filter(|v| not_deleted(v))
        .map(|item| {
            let options: i64 = item
                .get("options")
                .and_then(Value::as_array)
                .map_or(0, |opts| {
                    opts.iter()
                        .map(|o| {
                            let q = o.get("quantity").and_then(Value::as_i64).unwrap_or(1);
                            o.get("price").map_or(0, cents_of) * q
                        })
                        .sum()
                });
            let quantity = item
                .get("quantity")
                .and_then(Value::as_str)
                .and_then(parse_quantity)
                .unwrap_or(1);
            (item.get("price").map_or(0, cents_of) + options) * quantity
        })
        .sum();
    let charges: i64 = list("charges")
        .iter()
        .filter(|v| not_deleted(v))
        .map(|c| c.get("price").map_or(0, cents_of))
        .sum();
    let discounts: i64 = list("discounts")
        .iter()
        .filter(|v| not_deleted(v))
        .map(|d| d.get("price_off").map_or(0, cents_of))
        .sum();
    items + charges - discounts
}

/// The sum of an order's payments.
pub fn payments_total_cents(order: &Value) -> i64 {
    order
        .get("payments")
        .and_then(Value::as_array)
        .map_or(0, |p| {
            p.iter()
                .filter(|v| not_deleted(v))
                .map(|p| p.get("amount").map_or(0, cents_of))
                .sum()
        })
}

/// The names the control API accepts for [`template`].
pub const TEMPLATES: &[&str] = &[
    "deliveroo_prepaid",
    "ubereats_prepaid",
    "webshop_online",
    "webshop_cash_collection",
    "webshop_cash_delivery",
    "eat_in",
];

/// A ready-made order on the seed catalog.
pub fn template(name: &str) -> Option<OrderBuilder> {
    let customer = json!({
        "first_name": "Alex",
        "last_name": "Test",
        "phone": "+32400000000",
        "address_1": "Rue de Test 1",
        "postal_code": "4000",
        "city": "Liège",
    });
    let order = match name {
        // A platform order: delivered by the platform, paid on it,
        // no tax rates on the lines.
        "deliveroo_prepaid" => OrderBuilder::new("Deliveroo", "delivery")
            .item("MAKI-SAUMON-12", "Maki saumon", 1000, 2)
            .option("Sauce", "Soja sucrée", "SAUCE-SUCREE", 50)
            .item("COCA-33", "Coca-Cola 33cl", 300, 1)
            .paid_by("DELIVEROO", "Deliveroo")
            .field("collection_code", "DLV-1234")
            .customer(json!({ "first_name": "Alex", "last_name": "T." })),
        "ubereats_prepaid" => OrderBuilder::new("Uber Eats", "delivery")
            .item("CALIFORNIA-8", "California roll", 890, 1)
            .option("Suppléments", "Avocat", "EXTRA-AVOCAT", 100)
            .item("MISO-1", "Soupe miso", 350, 2)
            .paid_by("UBER_EATS", "Uber Eats")
            .customer(json!({ "first_name": "Sam" })),
        // The restaurant's own webshop, paid online before collection.
        "webshop_online" => OrderBuilder::new("Website", "collection")
            .item("MAKI-SAUMON-6", "Maki saumon", 550, 1)
            .tax_rate("6.0")
            .item("ASAHI-33", "Bière Asahi 33cl", 450, 1)
            .tax_rate("21.0")
            .paid_by("ONLINE", "Paiement en ligne")
            .customer(customer),
        // Paid in cash when collected.
        "webshop_cash_collection" => OrderBuilder::new("Website", "collection")
            .item("CALIFORNIA-8", "California roll", 890, 1)
            .tax_rate("6.0")
            .item("COCA-33", "Coca-Cola 33cl", 300, 2)
            .tax_rate("6.0")
            .customer(customer),
        // Delivered by the restaurant, paid in cash at the door.
        "webshop_cash_delivery" => {
            let b = OrderBuilder::new("Website", "delivery")
                .item("MAKI-SAUMON-12", "Maki saumon", 1000, 1)
                .tax_rate("6.0")
                .option("Sauce", "Soja", "SAUCE-SOJA", 0)
                .item("MISO-1", "Soupe miso", 350, 1)
                .tax_rate("6.0")
                .charge("DELIVERY", "Livraison", 250)
                .customer(customer);
            let total = order_total_cents(&b.order);
            b.payment("CASH", "Espèces", total)
        }
        // Ordered at the table through an ordering app, paid in it.
        "eat_in" => OrderBuilder::new("Table QR", "eat_in")
            .item("CALIFORNIA-8", "California roll", 890, 1)
            .tax_rate("12.0")
            .item("COCA-33", "Coca-Cola 33cl", 300, 1)
            .tax_rate("21.0")
            .paid_by("APP", "Paiement app"),
        _ => return None,
    };
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_catalog_parses_with_tax_rates() {
        let c = seed_catalog();
        assert_eq!(c.categories.len(), 3);
        let maki = &c.products[0];
        assert_eq!(maki.skus.len(), 2);
        assert_eq!(
            maki.tax_rate.as_ref().unwrap().eat_in.as_deref(),
            Some("12.0")
        );
        assert_eq!(c.charges[0].ref_field.as_deref(), Some("DELIVERY"));
    }

    #[test]
    fn totals_count_options_charges_and_discounts() {
        let order = OrderBuilder::new("X", "delivery")
            .item("A", "A", 300, 2)
            .option("L", "O", "O", 50)
            .charge("C", "C", 150)
            .discount("D", "D", 200)
            .build();
        // (3.00 + 0.50) x 2 + 1.50 - 2.00
        assert_eq!(order_total_cents(&order), 650);
    }

    #[test]
    fn every_template_is_built_and_prepaid_ones_are_fully_paid() {
        for name in TEMPLATES {
            let order = template(name).unwrap().build();
            let total = order_total_cents(&order);
            assert!(total > 0, "{name}");
            let paid = payments_total_cents(&order);
            match *name {
                "webshop_cash_collection" => assert_eq!(paid, 0, "{name}"),
                _ => assert_eq!(paid, total, "{name}"),
            }
        }
        assert!(template("nope").is_none());
    }
}
