# dummy_operation
This is dummy_model that represent some testing table for the idea of md model definition

## Type
operation

## Transformation
### Template
[dummy_template](template/dummy_template)
| Key | Value | Description |
| --- | --- | --- |
| `prop1` | `value1` | Description for prop1 |
| `prop2` | `value2` | Description for prop2 |
| `prop3` | `value3` | Description for prop3 |
### Code
```sql
SELECT *, {{session_id}} FROM {{dummy_model}} where f={{props__vata}}
```