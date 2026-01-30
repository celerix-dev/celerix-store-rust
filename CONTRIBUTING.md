# Contributing to Celerix Store Rust

First off, thank you for taking the time to contribute! It is people like you who make Celerix a better tool for everyone.

As a privacy-first, high-performance project, we have a few specific guidelines to ensure the application remains lean, fast, and secure.

---

## 🛡️ Our Privacy Manifesto
Before contributing, please remember:
* **No Telemetry:** We do not accept features that require tracking, analytics, or external data collection.
* **Local First:** Features should work offline whenever possible.
* **Data Sovereignty:** User data must stay on the user's machine.

## 🚀 Technical Standards
To maintain our "Performance Engineering" pillar, we follow these rules:
* **Rust Backend:** Keep the logic lean. Avoid heavy crates if a lighter alternative exists.
* **Secure Storage:** Prioritize data integrity and security for secrets and data.
* **Auditability:** Write clean, documented code. Since we are privacy-focused, users must be able to easily audit what the code is doing with their data.

## 🛠️ How Can I Contribute?

### Reporting Bugs
* Use the **GitHub Issues** tab.
* Describe the bug clearly and provide steps to reproduce it.
* Include your OS (Windows/macOS/Linux) and the version of Celerix you are using.

### Pull Requests
1. **Fork the repo** and create your branch from `main`.
2. **Lint your code:** Run `cargo fmt`.
3. **Rust Checks:** Ensure `cargo clippy` doesn't report any warnings.
4. **Description:** Explain what your PR does and why it's a good fit for Celerix Store Rust.

## ⚖️ License
By contributing to Celerix Store Rust, you agree that your contributions will be licensed under the **MIT License**.

---

Thank you for helping us build software that respects users!
