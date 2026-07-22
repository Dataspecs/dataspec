# dummy_model
This is dummy_model that represent some testing table for the idea of md model definition

## Type
model

## Tags
- vata
- vata2
- vata3

## Partition
### Column
dummy1
### Type
timestamp
### Granularity
day
### Range
| Key | Value |
| --- | --- |
| `start` | `0` |
| `end` | `100` |
| `interval` | `10` |
## Clusters
- dummy1
- dummy2

## Transformation
### Columns
#### dummy1
This is dummy column that represend dummy staff
##### Type
String
##### Labels
- a
- b
- c
##### Dependencies
- [dymmy_model2](../../models/dummy_model2#dummyA)
##### Tests
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
#### dummy2
This is dummy column that represend dummy staff
##### Type
Integer
##### Tests
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
### Tests
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
### Hooks
#### Pre
- [dummy_operation1](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
    | `prop2` | `value2` | Description for prop2 |
- [dummy_operation2](../operations/dummy_operation)
#### Post
- [dummy_operation3](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
- [dummy_operation4](../operations/dummy_operation)
#### Init
- [dummy_operation5](../operations/dummy_operation)
    | Key | Value | Description |
    | --- | --- | --- |
    | `prop1` | `value1` | Description for prop1 |
### Template
[dummy_template](../templates/dummy_template)
| Key | Value | Description |
| --- | --- | --- |
| `prop1` | `value1` | Description for prop1 |
| `prop2` | `value2` | Description for prop2 |
| `prop3` | `value3` | Description for prop3 |
### Code
```sql
SELECT *, {{session_id}} FROM {{dummy_model}} where f={{props__vata}}
```


## Default transformation
Optional name of transformation that runs by default if it's not set then default transformation is what is embedded into model