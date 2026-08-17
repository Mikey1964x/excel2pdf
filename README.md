# excel2pdf

Converts an Excel file to PDF, compatible with both Windows and Linux. Uses either LibreOffice or Microsoft Excel, with LibreOffice given priority if installed.

This repository provides **two independent implementations** of the same functionality:

- **Go** — a Go module (`github.com/Mikey1964x/excel2pdf/v2`) importable as a library in any Go project.
- **Rust** — a Rust crate (`excel2pdf`) publishable to [crates.io](https://crates.io) and importable in any Rust project.

Both versions support Windows, Linux, and macOS, prefer LibreOffice when installed, and fall back to Microsoft Excel (COM automation) on Windows.

---

## Go

### Add to your project

```sh
go get github.com/Mikey1964x/excel2pdf/v2
```

### Usage

```go
package main

import (
    "fmt"
    "github.com/Mikey1964x/excel2pdf/v2"
)

func main() {
    pdfFilePath, err := excel2pdf.ConvertExcelToPdf("file.xlsx")
    if err != nil {
        panic(err)
    }
    fmt.Println("PDF written to:", pdfFilePath)
}
```

---

## Rust

### Add to your project

Add the following to your `Cargo.toml`:

```toml
[dependencies]
excel2pdf = { git = "https://github.com/Mikey1964x/excel2pdf" }
```

Once the crate is published to [crates.io](https://crates.io), you can use a version number instead:

```toml
[dependencies]
excel2pdf = "0.1"
```

### Usage

```rust
fn main() {
    // Optionally set the maximum number of concurrent conversions (default is 3).
    excel2pdf::set_max_concurrency(4);

    // Convert a single Excel file to PDF.
    match excel2pdf::convert_excel_to_pdf("file.xlsx") {
        Ok(pdf_path) => println!("PDF written to: {}", pdf_path.display()),
        Err(e) => eprintln!("Conversion failed: {}", e),
    }
}
```

### Merging multiple PDFs

The Rust crate also exposes a `combine_pdfs` function for merging multiple PDF files into one:

```rust
use std::path::Path;

fn main() {
    let inputs = vec!["report1.pdf", "report2.pdf", "report3.pdf"];
    let output = Path::new("merged.pdf");

    match excel2pdf::combine_pdfs(&inputs, output) {
        Ok(path) => println!("Merged PDF written to: {}", path.display()),
        Err(e) => eprintln!("Merge failed: {}", e),
    }
}
```

> **Note:** Only one merge operation may run at a time. If a merge is already in progress, `Excel2PdfError::AlreadyProcessing` is returned immediately.

---

## Platform support

| Feature                  | Go | Rust |
|--------------------------|----|------|
| Windows (LibreOffice)    | ✅ | ✅   |
| Windows (Microsoft Excel)| ✅ | ✅   |
| Linux (LibreOffice)      | ✅ | ✅   |
| macOS (LibreOffice)      | ✅ | ✅   |
| Merge PDFs               | ✅ | ✅   |
