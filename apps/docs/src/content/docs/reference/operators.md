---
title: Expressions and operators
description: Check expressions and every operator.
---

## Expressions

| Expression | Selects |
| --- | --- |
| `res.status` | Status code |
| `res.responseTime` | Time in ms |
| `res.headers.<name>` | Header (case-insensitive) |
| `res.headers['<name>']` | Header with a dash or symbol in its name |
| `res.body` | JSON body |
| `res.body.a.b[0].c` | Path into the body |
| `res.body['a b']` | Key containing spaces |
| `a.b`, `$.a.b` | Same as `res.body.a.b` |

## Operators

| Operator | Passes when the value… |
| --- | --- |
| `eq`, `neq` | equals / doesn't equal |
| `gt`, `gte`, `lt`, `lte` | compares as a number |
| `in`, `notIn` | is / isn't in `[a, b]` or `a, b` |
| `contains`, `notContains` | contains an item, a key, or a substring |
| `startsWith`, `endsWith` | starts / ends with the text |
| `matches`, `notMatches` | matches a regex: `^v\d+` or `/^v\d+/` |
| `length` | has this many characters, items or keys |
| `isDefined`, `isUndefined` | exists / doesn't exist |
| `isNull`, `isNotNull` | is / isn't `null` |
| `isEmpty`, `isNotEmpty` | has length 0 / more than 0 |
| `isString`, `isNumber`, `isBoolean`, `isArray` | is that type |
| `isJson` | is an object or array |
| `isTruthy`, `isFalsy` | is truthy / falsy |

`is…` operators take no value.

If the expression matches nothing, these pass: `neq`, `notIn`, `notContains`, `notMatches`, `isUndefined`, `isFalsy`. All other operators fail.

## Values

| `value` | Read as |
| --- | --- |
| `"200"` | number |
| `"true"` | boolean |
| `'"200"'` | string |
| `"[1, 2]"` | array |
| `ok` | string |
