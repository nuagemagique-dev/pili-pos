#!/usr/bin/env python3
"""The demo menu as a HubRise catalog, for the HubRise mock of the demo.

Usage: make_hubrise_catalog.py <pos.db> > hubrise_catalog_tokyosushi.json

<pos.db> is a database built from seed_demo_tokyosushi.sql
(PILI_POS_DATA_DIR=<dir> target/debug/pili-pos --init-db --seed <seed>).
Every category and product keeps the HubRise ref the seed gives it
(TSB-CAT-<id>, TSB-<id>), so the catalog sync of the till updates the
same rows: names (fr, en, zh, in the custom_fields encoding of
tsb-service), menu codes (custom_fields.code), prices, VAT (dine-in and
takeaway rates) and the drinks that skip the kitchen ticket.
"""
import json
import sqlite3
import sys

RATES = {"A": "21.0", "B": "12.0", "C": "6.0", "D": "0.0"}


def eur(cents):
    return f"{cents // 100}.{cents % 100:02d} EUR"


def names(db, kind, entity_id):
    rows = db.execute(
        "SELECT lang, value FROM translations WHERE entity_type = ? AND entity_id = ? AND field = 'name'",
        (kind, entity_id),
    ).fetchall()
    return {lang: value for lang, value in rows}


def main():
    db = sqlite3.connect(sys.argv[1])
    categories, products = [], []
    for cat_id, cat_ref in db.execute(
        "SELECT id, external_id FROM categories WHERE active = 1 ORDER BY sort_order, id"
    ).fetchall():
        n = names(db, "category", cat_id)
        payload = {"translations": {lang: n.get(lang, "") for lang in ("fr", "en", "nl", "zh")}}
        categories.append({"ref": cat_ref, "name": n["fr"], "description": json.dumps(payload, ensure_ascii=False)})
        for item_id, ref, price, vat, vat_takeaway, kitchen, code in db.execute(
            "SELECT id, external_id, price_cents, vat_code, COALESCE(vat_code_takeaway, vat_code), kitchen_printer, code "
            "FROM menu_items WHERE category_id = ? AND active = 1 ORDER BY sort_order, id",
            (cat_id,),
        ).fetchall():
            n = names(db, "menu_item", item_id)
            custom = {"translations": {lang: {"name": value} for lang, value in sorted(n.items())}}
            if code:
                custom["code"] = code
            sku = {"ref": ref, "price": eur(price), "option_list_refs": [], "custom_fields": custom}
            if not kitchen:
                sku["tags"] = ["pos:no_kitchen"]
            products.append({
                "ref": f"P-{ref}",
                "category_ref": cat_ref,
                "name": n["fr"],
                "tax_rate": {"eat_in": RATES[vat], "collection": RATES[vat_takeaway], "delivery": RATES[vat_takeaway]},
                "skus": [sku],
            })
    json.dump({"categories": categories, "products": products, "option_lists": [], "charges": [],
               "deals": [], "discounts": [], "variants": []}, sys.stdout, ensure_ascii=False, indent=1)
    sys.stdout.write("\n")


main()
