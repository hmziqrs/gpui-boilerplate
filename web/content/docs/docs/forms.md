---
title: Forms
description: Validated forms with gpui-form and koruma
---

## Overview

gpui-starter includes a working registration form that demonstrates:

- **gpui-form** — derive macros that generate form UI and state management
- **koruma** — validation rules with composable builders
- **es-fluent** — localized error messages

The form is in `src/views/form_page.rs`.

## Defining a form model

The `RegistrationForm` struct derives multiple macros:

```rust
#[derive(Clone, Debug, Default, EsFluentVariants, GpuiForm, Koruma, KorumaAllFluent)]
#[fluent_variants(keys = ["description", "label"])]
#[gpui_form(koruma(fluent))]
pub struct RegistrationForm {
    #[gpui_form(component(input))]
    #[koruma(NonEmptyValidation::<_>::builder())]
    pub name: String,

    #[gpui_form(component(input))]
    #[koruma(EmailValidation::<_>::builder())]
    pub email: String,

    #[gpui_form(component(input))]
    #[koruma(NonEmptyValidation::<_>::builder())]
    pub password: String,

    #[gpui_form(component(input))]
    #[koruma(PhoneNumberValidation::<_>::builder())]
    pub phone: String,

    #[gpui_form(component(input))]
    #[koruma(UrlValidation::<_>::builder())]
    pub website: String,
}
```

## Validation rules

The following validators from `koruma-collection` are used:

| Validator | Purpose | Example |
|-----------|---------|---------|
| `NonEmptyValidation` | Requires non-empty input | Name, password |
| `EmailValidation` | Validates email format | `user@example.com` |
| `PhoneNumberValidation` | Validates phone format | `(555) 123-4567` |
| `UrlValidation` | Validates URL format | `https://example.com` |

All validators use the builder pattern for configuration.

## Fluent validation messages

The `#[fluent_variants(keys = ["description", "label"])]` attribute generates typed label and description variants. Combined with `KorumaAllFluent`, validation errors are automatically localized.

Labels are defined in the `.ftl` files:

```ftl
registration_form_label_variants-name = Full Name
registration_form_label_variants-email = Email
registration_form_description_variants-email = We'll never share your email with anyone else.
```

## Building the form UI

Use `gpui-component`'s `v_form()` and `field()` builders:

```rust
v_form()
    .label_width(px(160.))
    .child(
        field()
            .label(crate::i18n::localize_message(
                &RegistrationFormLabelVariants::Name,
            ))
            .required(true)
            .description_fn({
                // closure that renders description + errors
            })
            .child(Input::new(&self.fields.name_input)),
    )
```

## Handling submission

Validate on submit and show errors or success:

```rust
Button::new("submit")
    .primary()
    .label("Create Account")
    .on_click(cx.listener(|this, _, window, cx| {
        this.touched = true;
        let valid = this.current_data.validate().is_ok();
        if valid && this.agree_terms {
            this.submitted = true;
            window.push_notification("Form submitted successfully!", cx);
        }
    }))
```

Validation runs via `self.current_data.validate()` which returns `Result<(), ValidationErrors>`. Errors are then extracted per field and displayed inline.
