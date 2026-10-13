# Septuvo Brand Kit

**Septuvo — Security Without Compromise.**

The official Septuvo visual identity package contains logos, product marks, application icons, color tokens, social graphics, reference boards, and brand usage guidelines. These assets provide a consistent visual foundation for Septuvo and the Septuvo VPN, Septuvo Protocol (SVP), and Septuvo Server product family.

![Septuvo primary mark](02%20-%20Logos/PNG_Transparent/Septuvo_Primary_Dark_Transparent.png)

## Brand guidelines

**[Septuvo Brand Identity and Asset Guidelines (PDF)](Septuvo_Brand_Guidelines.pdf)** is the primary visual reference for the identity system, usage rules, typography, color palette, and product family. Read it before adapting these assets to a new application or surface.

## Asset directory

| Directory | Contents | Typical use |
| --- | --- | --- |
| `01 - Reference Boards/` | Seven visual reference sheets covering the brand overview, logo system, colors, iconography, usage, and applications | Brand consistency and visual reference |
| `02 - Logos/PNG_Transparent/` | Full logos, horizontal wordmarks, and standalone S marks with transparent backgrounds | Websites, applications, documentation, and print layouts |
| `02 - Logos/SVG_Vector_Interpretations/` | Editable vector outlines, wordmarks, and standalone S symbols | Scalable graphics and integrations that require SVG |
| `03 - App Icons/` | Windows ICO, favicon PNG/SVG, and Septuvo, VPN, Protocol, and Server app icons | Desktop apps, browser tabs, mobile and web application surfaces |
| `04 - Colors And Tokens/` | Color-token documentation, JSON, CSS, and Rust constants | Product interfaces, design systems, and websites |
| `05 - Social And Wallpapers/` | Social sharing image, banner, and desktop wallpaper | Project profiles, posts, websites, and presentations |
| `06 - Product Family/` | Septuvo VPN, Septuvo Protocol, and Septuvo Server transparent product lockups | Product identity and cross-product navigation |

### Choosing a logo

- **Dark backgrounds:** `02 - Logos/PNG_Transparent/Septuvo_Primary_Dark_Transparent.png`
- **Light backgrounds:** `02 - Logos/PNG_Transparent/Septuvo_Primary_Light_Transparent.png`
- **Horizontal wordmark:** Select the corresponding dark or light wordmark in `PNG_Transparent/`.
- **Compact spaces:** Use the supplied `Septuvo_S_*` symbol assets and a size-specific app icon.
- **Vector workflows:** Use the SVG assets when editable geometry and scaling are needed. The vector ribbon artwork is an interpretation of the detailed raster mark; the PNG masters preserve the metallic texture and should be preferred where precise visual fidelity matters.

Use the supplied dimensions and proportions. Do not stretch, arbitrarily recolor, or place marks against visually cluttered backgrounds. Choose assets for the actual background rather than relying on automatic color inversion.

### App icons

The `03 - App Icons/Septuvo/` directory contains dark and light icon variants at common sizes from 16 px through 1024 px. Product subdirectories provide corresponding assets for the VPN, Protocol, and Server identities. The root of `03 - App Icons/` also contains:

- `Septuvo_Windows.ico` for Windows application packaging.
- `favicon.svg` and `favicon-16.png`, `favicon-32.png`, `favicon-48.png`, `favicon-64.png` for websites.
- `septuvo-icon-64.png` and `septuvo-icon-256.png` for convenient general-purpose app references.

Select a purpose-built small icon at 16-32 px rather than scaling down an elaborate logo or banner.

## Brand colors

The canonical token values are documented in [`04 - Colors And Tokens/brand-tokens.md`](04%20-%20Colors%20And%20Tokens/brand-tokens.md) and provided in ready-to-use [`septuvo-colors.json`](04%20-%20Colors%20And%20Tokens/septuvo-colors.json), `septuvo-theme.css`, and `septuvo-theme.rs`.

| Role | Value |
| --- | --- |
| Primary Blue | `#0066FF` |
| Electric Cyan | `#00D5F8` |
| Accent Teal | `#00E5B8` |
| Accent Purple | `#A855F7` |
| Midnight | `#050A14` |
| Navy | `#0B1220` |
| Surface | `#1E293B` |
| Muted Text | `#94A3B8` |
| Primary Light Text | `#F8FAFC` |

Additional border, sky, aqua, and status colors appear in the token files. Reference-board gradients and metallic reflections are illustrative treatments and need not correspond to a single solid token.

## Typography

**Inter** is the supporting typeface for application interfaces, headings, and documentation. Use legible weights and sufficient contrast, especially for small labels. The included logo wordmarks are dedicated artwork, not a substitute for normal editable interface text.

Font files are **not** included in the kit; obtain and use Inter under its own applicable license.

## Accessibility and accurate messaging

- Preserve minimum text contrast and clear space around every logo.
- Pair color-coded statuses with understandable text: never rely on an icon or color alone.
- Distinguish an activated network adapter from an authenticated, working VPN connection.
- Do not imply that a product has undergone an independent security audit or received certification unless supported by evidence.
- The brand kit defines visual identity; inclusion of product artwork does not certify the availability, security, or capabilities of software products.
- Use trademark symbols and legal notices only when appropriate to the project's rights and actual registrations.

## Using this kit in GitHub

Place the contents of this archive directly inside the repository's **`brand/`** directory. Keep `README.md` and `Septuvo_Brand_Guidelines.pdf` at that folder's root, followed by the six numbered asset directories. The relative links in this README are written for that layout.

When updating an existing `brand/` directory, replace superseded files rather than retaining duplicate or outdated versions. Preserve original PNG and SVG dimensions where possible, and keep machine-readable token values synchronized across JSON, CSS, and Rust.

---

**Septuvo brand identity · October 2026**
