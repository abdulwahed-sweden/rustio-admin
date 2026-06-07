---
artifact: DESIGN_ARCHITECTURE
layer: what
status: active
updated: 2026-06-07
---

# Design Architecture — WHAT

> The structure between intent and tokens. Reasoned from the Brief; it constrains
> what the token layer must support.

## Information Architecture

Nine models group into three operator-facing domains:

- **Catalogue** — `Product`, `Category`, `ProductImage`. What the store sells.
- **Customers** — `Customer`, `Address`. Who buys, and where things ship.
- **Sales** — `Order`, `OrderItem`, `Payment`, `CartItem`. The money path.

- **Primary entities (and why):** `Order` (the daily work), `Product` (the asset
  being managed), `Customer` (the relationship). These three lead everything.
- **Relationships that must be legible in the UI:** `Order → Customer`,
  `Order → OrderItem → Product`, `Product → Category`, `Customer → Address`,
  `Order → Payment`. Inline relations make these visible without navigation.
- **Priority order:** Sales first (Orders, Payments), then Catalogue, then
  Customers. Secondary join tables (`OrderItem`, `CartItem`) surface inline, not
  as top-level destinations.

## Navigation Structure

```
Dashboard
Catalogue
  ├─ Products
  └─ Categories
Customers
  └─ Customers
Sales
  ├─ Orders
  └─ Payments
```

- **Top-level sections (ordered):** Dashboard · Catalogue · Customers · Sales.
- **Primary entities in the sidebar:** Products, Categories, Customers, Orders,
  Payments — five focused destinations.
- **Buried (reached through their parent, and by URL — not in nav):** `OrderItem`,
  `CartItem`, `ProductImage`, and `Address`. Addresses open through a Customer.
  (Decision D-008 / R-008.)
- **Terminology / labels:** the operator's words — "Orders", "Customers",
  "Catalogue" — never the schema's join-table names.

## UX Hierarchy

- **List pages — emphasis & primary action:** the record link leads; status reads
  at a glance (navy/amber/semantic); the primary action is "create".
- **Detail pages — what leads:** an order/customer summary header first, then
  inline related records (line items, payments, addresses).
- **Forms — grouping & the one primary action:** related fields grouped; a single
  navy primary button; destructive actions kept visually quiet.
- **Empty / error / loading states:** calm and instructive, never alarming; amber
  for attention, semantic red reserved for genuine danger.
