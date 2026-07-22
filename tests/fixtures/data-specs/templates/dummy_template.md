# dummy_template
This is dummy_template

## Type
template

## Transformation
| Key | Value | Description |
| --- | --- | --- |
| `table_name` | `` | Target table name |
| `code` | `` | Inner SELECT passed from the parent |
### Code
```sql
create or replace table `{{props__table_name}}__tmp` as (
    {{props__code}}
);
```
