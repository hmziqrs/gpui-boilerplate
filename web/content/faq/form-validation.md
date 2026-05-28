---
question: "How does form validation work?"
description: "gpui-starter uses gpui-form derive macros with koruma validation rules for type-safe form handling."
category: "Features"
order: 8
---

Forms use the `gpui-form` crate with derive macros and `koruma` validation rules. This gives you type-safe form handling with compile-time guarantees.

## Defining a form

```rust
use gpui_form::Form;
use koruma::{Validate, Required, MinLength};

#[derive(Form, Validate)]
struct LoginForm {
    #[validate(Required)]
    username: String,

    #[validate(Required, MinLength(8))]
    password: String,
}
```

## Rendering

The form component handles validation automatically:

```rust
FormView::new(LoginForm::default())
    .on_submit(|data, cx| {
        // data is validated — username and password are guaranteed non-empty
        login(data.username, data.password, cx);
    })
```

Validation errors appear inline next to the relevant fields. No manual error tracking needed.

See the [forms documentation](/docs/forms/) for the full API reference.
