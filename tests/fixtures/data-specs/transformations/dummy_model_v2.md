# dummy_model_v2
Dummy model transformation version 2

## Type
transformation

## Model
[dummy_model](../models/dummy_model)

## Columns
### dummy1
This is dummy column that represend dummy staff
#### Type
String
#### Labels
- a
- b
- c
#### Dependencies
- [dymmy_model2](../../models/dummy_model2#dummyA)
#### Tests
- [dummy_test_column](../tests/dummy_test)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
- [dummy_test_column2](../tests/dummy_test2)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
### dummy2
This is dummy column that represend dummy staff
#### Type
Integer
#### Tests
- [dummy_test_column](../tests/dummy_test)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
- [dummy_test_column2](../tests/dummy_test2)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
## Tests
- [dummy_test](../tests/dummy_test)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
- [dummy_test2](../tests/dummy_test2)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
    | `prop3` | `value3` | Description for prop3 |
## Hooks
### Pre
- [dummy_operation1](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
- [dummy_operation2](../operations/dummy_operation)
### Post
- [dummy_operation3](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
- [dummy_operation4](../operations/dummy_operation)
### Init
- [dummy_operation5](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
## Template
[dummy_template](../templates/dummy_template)
| Key | Value | Description |
| --- | --- | --- |
| `prop1` | `value1` | Description for prop1 |
| `prop2` | `value2` | Description for prop2 |
| `prop3` | `value3` | Description for prop3 |
## Code
```sql
SELECT *, {{session_id}} FROM {{dummy_model}} where f={{props__vata}}
```