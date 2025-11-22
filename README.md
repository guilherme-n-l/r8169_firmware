# r8169 firmware in Rust

> Created using [Rust out-of-tree module](https://github.com/Rust-for-Linux/rust-out-of-tree-module). For further information, please use the former as reference guide.

This repository is dedicated to the development of firmware for Realtek Ethernet Controllers (devices supported by the `r8169` driver) with a distinct and exclusive focus on the Rust programming language.

### Project Scope and Objective

This project's objective is to leverage Rust's capabilities in low-level, embedded development to create a modern, high-performance, and memory-safe implementation of the Realtek NIC firmware.

* Target: Realtek Ethernet NICs (RTL8169/8168/8111 series, etc.).
* Core Goal: To develop the firmware component of the r8169 driver.

### Development Focus: Rust

The entire development effort and all future contributions to this repository are centered on writing code in Rust.

Motivation: The primary goal for choosing Rust is to evaluate the feasibility and process of porting Linux kernel components—specifically low-level driver logic—to a memory-safe language using a gradual, component-by-component approach. Rust is selected for its guarantees on memory safety and its zero-cost abstractions, which allows it to match the performance required for critical, performance-sensitive firmware while inherently eliminating common vulnerabilities found in the original C-based solutions.

### C Code as the Functional Specification

The repository includes the complete C source code from the **v6.17.8** Linux kernel `r8169` driver.

* Role: This C code is not part of the active development process and is only intended for compilation within this project.
