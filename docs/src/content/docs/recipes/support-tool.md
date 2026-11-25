---
title: A support tool in a flow
description: Prompts, a pick list and a confirmation around a few admin requests.
---

```yaml title="critters/support/flows/resend-receipt.yml"
name: Resend a receipt
steps:
  - tool: prompt
    vars:
      - name: email
        label: Customer email

  - request: admin/customers/search.yml
    with: { q: "{{email}}" }
    expect: 200
    capture:
      customerId: res.body.results[0].id
      customerName: res.body.results[0].name

  - request: admin/orders/list.yml
    with: { customerId: "{{customerId}}" }
    capture:
      orders: res.body.orders

  - tool: pick
    title: Which order?
    list: "{{orders}}"
    label: summary
    set:
      orderId: id
      orderNumber: number

  - tool: display
    title: Resend receipt for #{{orderNumber}}?
    text: To {{customerName}} <{{email}}>
    pause: true

  - request: admin/orders/resend-receipt.yml
    with: { orderId: "{{orderId}}" }
    expect: 202

  - tool: notify
    title: Receipt sent
```

The `support` critter holds the support account's credentials and a `login` flow.

From the CLI:

```sh
cactus-cli flow . support resend-receipt --var email=jo@example.com --var orderNumber=10432
```

`--var orderNumber=10432` answers the pick by choosing the order whose `number` is `10432`.

## Similar flows

- **Impersonate a user**: prompt for an email, call the impersonation endpoint, `open` the returned link.
- **Extend a trial**: prompt for an account, `pick` a plan, post the change.
- **Retry a failed job**: list failed jobs, `pick` one, `display` its error, retry it.
- **Seed a demo account**: `set` a `$randomEmail`, sign up, create sample data, `display` the login.
