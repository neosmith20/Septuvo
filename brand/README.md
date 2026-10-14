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
| `02 - Logos/PNG_Transparent/` | Primary, horizontal, vertical, monochrome, and wordmark logos in dark and light versions, plus standalone S marks, all with transparent backgrounds | Websites, applications, documentation, and print layouts |
| `02 - Logos/SVG_Vector_Interpretations/` | Scalable vector interpretations of the primary, horizontal, vertical, and monochrome logos (dark and light) and of the standalone S marks | Scalable graphics and integrations that require SVG |
| `03 - App Icons/` | Windows ICO, favicon PNG/SVG, general-purpose icons, and Septuvo, VPN, Protocol, and Server app icons | Desktop apps, browser tabs, mobile and web application surfaces |
| `04 - Colors And Tokens/` | Color-token documentation, JSON, CSS, and Rust constants | Product interfaces, design systems, and websites |
| `05 - Social And Wallpapers/` | Social sharing images (1200 x 630 and 1080 x 1080), banner (1500 x 500), header (2560 x 1080), and desktop wallpapers (1920 x 1080 and 3840 x 2160) | Project profiles, posts, websites, and presentations |
| `06 - Product Family/` | Septuvo VPN, Septuvo Protocol, and Septuvo Server lockups in horizontal and vertical layouts, each in dark and light versions, with transparent backgrounds | Product identity and cross-product navigation |

### Choosing a logo

- **Dark backgrounds:** `02 - Logos/PNG_Transparent/Septuvo_Primary_Dark_Transparent.png`
- **Light backgrounds:** `02 - Logos/PNG_Transparent/Septuvo_Primary_Light_Transparent.png`
- **Wide, short spaces:** `Septuvo_Horizontal_Dark_Transparent.png` or `Septuvo_Horizontal_Light_Transparent.png`.
- **Tall or square spaces:** `Septuvo_Vertical_Dark_Transparent.png` or `Septuvo_Vertical_Light_Transparent.png`.
- **Without the tagline:** `Septuvo_Wordmark_Dark_Transparent.png` or `Septuvo_Wordmark_Light_Transparent.png` show the S mark and the Septuvo name only.
- **Single-color artwork:** `Septuvo_Monochrome_Dark_Transparent.png` or `Septuvo_Monochrome_Light_Transparent.png`.
- **Compact spaces:** Use the standalone S marks (`Septuvo_S_Metallic_Transparent.png`, `Septuvo_S_Navy_Transparent.png`, `Septuvo_S_White_Transparent.png`) and a size-specific app icon.
- **Vector workflows:** Use the SVG assets when scalable geometry is needed. The vector ribbon artwork is an interpretation of the detailed raster mark; the PNG masters preserve the metallic texture and should be preferred where precise visual fidelity matters. There is no wordmark-only SVG.

Use the supplied dimensions and proportions. Do not stretch, arbitrarily recolor, or place marks against visually cluttered backgrounds. Choose assets for the actual background rather than relying on automatic color inversion.

### App icons

The icons are supplied in these sizes:

| Folder | Dark icons | Light icons |
| --- | --- | --- |
| `03 - App Icons/Septuvo/` | 16, 24, 32, 48, 64, 128, 256, 512, 1024 px | 128, 256, 512, 1024 px |
| `03 - App Icons/VPN/` | 64, 128, 256, 512, 1024 px | 128, 256, 512, 1024 px |
| `03 - App Icons/Protocol/` | 64, 128, 256, 512, 1024 px | 128, 256, 512, 1024 px |
| `03 - App Icons/Server/` | 64, 128, 256, 512, 1024 px | 128, 256, 512, 1024 px |

Light icons are not supplied below 128 px, and the VPN, Protocol, and Server icons are not supplied below 64 px. The root of `03 - App Icons/` also contains:

- `Septuvo_Windows.ico` for Windows application packaging (16, 24, 32, 48, 64, 128, and 256 px).
- `favicon.svg` and `favicon-16.png`, `favicon-32.png`, `favicon-48.png`, `favicon-64.png` for websites.
- `septuvo-icon-64.png` and `septuvo-icon-256.png` for convenient general-purpose app references.

For compact interfaces at 16-32 px, use the supplied dark Septuvo icons in those sizes rather than scaling down an elaborate logo or banner.

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

## Keeping the folder clean

Do not commit operating-system clutter such as `Thumbs.db`, `Desktop.ini`, or `.DS_Store` files. The repository's root `.gitignore` excludes them.

---

**Septuvo brand identity · October 2026**
