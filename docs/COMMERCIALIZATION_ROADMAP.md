# Commercialization Roadmap

Verified against the linked platform and regulatory guidance on 2026-07-15. This is an operating plan, not legal, tax, or certification advice.

## Offer

- One Lofi box: **$20**.
- Three-box pack: **$50**.
- Shipping and sales tax are additional.
- Initial market: direct-to-consumer United States only.
- V1 should be USB-C powered, with no battery and no power adapter included.

The bundle is a 16.7% per-unit discount and should be the hero offer. It also demonstrates the product's actual advantage: three boxes can divide pulse, low, color, pocket, and motif roles instead of acting like three copies of one speaker.

Do not offer wholesale at this retail price. A conventional retailer margin would consume the remaining product margin.

## Current Readiness

The repository is a strong desktop proof of the difficult software ideas:

- deterministic procedural music
- shared device runtime
- leaderless-emergent mesh clock with convergence tests
- display framebuffer
- simulated loss, latency, drift, split, and merge
- realtime multi-device browser lab and WAV rendering

It is not yet a sellable hardware product. There is no firmware crate, schematic, PCB, enclosure, production BOM, physical latency measurement, factory test fixture, regulatory report, or field-return data. Marketing may start with audience research and a no-payment waitlist; sales must wait for the hardware gates below.

## Unit Economics

The three-pack sets the real cost ceiling because it earns only $16.67 per device. Using Stripe's current standard US online-card rate of 2.9% + $0.30 as a planning baseline ([Stripe pricing](https://stripe.com/pricing)):

| Per order | One box | Three boxes |
| --- | ---: | ---: |
| Revenue | $20.00 | $50.00 |
| Payment fee | $0.88 | $1.75 |
| Landed product COGS | $7.50 | $22.50 |
| Pick and packaging | $1.25 | $2.00 |
| Defect/return reserve | $0.60 | $1.50 |
| Contribution before shipping, overhead, and acquisition | **$9.77** | **$22.25** |
| Contribution margin | **48.9%** | **44.5%** |

These are targets, not supplier quotes. "Landed product COGS" includes parts, PCB assembly, enclosure, labels, inbound freight, duties, assembly labor, programming, and normal yield loss. Customer-paid outbound shipping is required at these prices. At $10 landed COGS, bundle contribution falls below 30%, which is too thin for paid acquisition and warranty surprises.

Hard financial gates:

- Landed COGS at the first repeatable production quantity is **$7.50 or less per device**.
- First-pass manufacturing yield is at least **95%**.
- Defect/return rate from the pilot is below **3%**.
- The three-pack remains at least **40% of orders**.
- Do not scale paid ads until contribution after acquisition remains positive by cohort.

## V1 Cost Architecture

Design the hardware from the retail constraint backward:

- Use an ESP32-S3 module with an existing modular approval and its approved antenna configuration.
- Use one PCB, one button, one small SSD1306 display, one I2S class-D speaker path, and one commodity speaker.
- Power over USB-C at 5 V. Excluding a battery removes charging, cell sourcing, battery shipping, and much of the safety burden.
- Do not include a cable or wall adapter; disclose what the customer needs.
- Prefer a snap-fit or two-screw enclosure designed for fast assembly.
- Add programming/test pads and a production self-test from the first PCB revision.
- Give each unit a stable device id and an explicit group-pairing flow. Lowest-id mesh discovery alone could merge unrelated nearby products.
- Keep music generation local and cloud-free. This avoids an account system, recurring infrastructure cost, and unnecessary personal data.

If quotes cannot meet the cost ceiling, change the product or the price before taking orders. Do not recover a structurally bad BOM through unpaid assembly labor.

## Product Gates

### Gate 1: Product Definition (weeks 1-2)

- Freeze the promise in one sentence: "A tiny box that makes an endless lo-fi part; put three together and they form a band."
- Lock V1 controls, ports, power requirements, dimensions, included items, and non-features.
- Decide whether the display is essential after testing a physical appearance mockup. Preserve it only if it materially improves purchase intent.
- Build a should-cost BOM with target, quoted, and worst-case columns.
- Interview 15-20 target buyers using video and nonfunctional mockups. Ask what they think it is and what would stop a $20/$50 purchase; do not pitch around confusion.

Exit: at least 10 target buyers can explain the one-versus-three value without prompting, and the should-cost BOM is at or below $7.50.

### Gate 2: Engineering Validation (weeks 3-8)

- Add `lofi-firmware-esp32s3` and a board crate.
- Prove I2S DMA audio with no allocation or blocking in the render path.
- Drive the real SSD1306 framebuffer and button state through `lofi-app::Device`.
- Connect ESP-NOW packets to `lofi_core::mesh::SyncEngine`.
- Add pairing, persisted identity/settings, watchdog recovery, and brownout behavior.
- Produce 20 hand-built EVT units on the intended electrical architecture.
- Measure audio start latency, device-to-device timing, power, thermals, RF range, and eight-hour playback stability.

Exit: 20 units complete an eight-hour test; a three-box group stays musically aligned under ordinary home RF conditions; no known failure can damage the device or attached USB supply.

### Gate 3: Design Validation and Compliance (weeks 9-14)

- Revise the PCB and enclosure from EVT results; freeze the production BOM.
- Build 50-100 DVT units using the intended assembler and process.
- Create a fixture that tests button, display, speaker, current draw, radio, device id, and firmware version in under 60 seconds.
- Run drop, connector-cycle, ESD pre-scan, thermal, sustained-playback, and neighboring-swarm tests.
- Engage an accredited compliance lab before freezing the RF layout. A certified module can reduce transmitter work, but the host product still needs the correct antenna/layout, exterior "Contains FCC ID" treatment when applicable, and evaluation of the finished digital device. FCC modular-label guidance is described in [FCC 07-56](https://docs.fcc.gov/public/attachments/FCC-07-56A1.pdf).
- Obtain product liability insurance and review the user instructions, labels, warnings, refund policy, and limited warranty with qualified advisers.

Exit: signed-off DVT report, compliance path/report, production files, supplier lead times, packaging, test fixture, and landed COGS at or below the gate.

### Gate 4: Pilot Drop (weeks 15-18)

- Sell 100-250 numbered units from inventory, not an open-ended preorder.
- State a shipment window supported by finished stock and fulfillment capacity.
- Hold back 3% of units for immediate replacements.
- Track every unit by batch, firmware, test result, ship date, support contact, and disposition.
- Personally review the first 25 setup sessions and every return reason.

Exit after 30 days: at least 95% shipped on time, less than 3% defective/returned for product faults, support burden below 10 minutes per order, and positive contribution after refunds and acquisition.

### Gate 5: Repeatable Drops (months 5-8)

- Fix pilot failures before ordering the next batch.
- Increase quantities only one step at a time: 100, 250, 500, then 1,000.
- Add colorways only when they share electronics, firmware, test, and packaging.
- Reorder when sell-through and supplier lead time justify it; do not manufacture from follower count.

## Instagram Sales System

Use Instagram for discovery and proof, with an owned checkout as the source of truth. Meta says Shops/product tagging require Commerce Eligibility compliance and availability in a supported country, and notes that some shop/checkout features are no longer supported ([Instagram supported countries](https://www.facebook.com/help/instagram/321000045119159/)). Treat product tagging as optional distribution, not critical infrastructure.

Initial setup:

- Professional Instagram account, matching Facebook Page, real business identity, support email, and owned domain.
- A simple storefront with two SKUs, inventory control, tax, shipping, order email, refund flow, analytics, and warranty link. Shopify Starter is explicitly aimed at social-link selling ([Shopify Starter](https://help.shopify.com/en/manual/intro-to-shopify/pricing-plans/plans-features/shopify-starter-plan)); Stripe Payment Links is a lean alternative for validation.
- Bio states the literal product and offer. Link goes directly to the two-SKU purchase page, not a general link directory.
- Pinned posts: what one box does, what three boxes do, and current availability/shipping date.
- Highlights: `Sound`, `1 vs 3`, `Setup`, `Shipping`, and `Support`.

Content should let the product demonstrate itself:

- Four short Reels per week: one box starts, then boxes two and three join audibly and visibly.
- One build/process post per week showing real boards, assembly, testing, or packing.
- One customer-performance post per week once pilot units exist, with permission.
- Daily Stories during a drop for inventory, packing, support answers, and buyer clips.
- One live session per drop that starts from unplugged products and shows setup without edits.

Use original device audio. Avoid generic "lo-fi lifestyle" footage that hides the object or makes synchronization impossible to verify.

Launch sequence:

1. Weeks 1-8: publish prototypes and collect email waitlist signups without payment.
2. Weeks 9-14: seed 10-20 DVT units to small music-making creators; require a clear disclosure of the free product and do not script praise ([FTC influencer disclosures](https://www.ftc.gov/business-guidance/resources/disclosures-101-social-media-influencers)).
3. Two weeks before stock: announce quantity, price, exact included items, ship window, and return policy.
4. Launch day: publish the one-to-three demo, email the waitlist, enable the two SKUs, and show remaining stock truthfully.
5. After sellout: return to a no-payment waitlist until the next finished batch is scheduled.

Internal funnel targets for the pilot, to be replaced by observed baselines:

- Reel view to profile visit: at least 1.5%.
- Profile visit to store session: at least 15%.
- Store session to purchase: at least 3%.
- Waitlist to purchase in seven days: at least 5%.
- Three-pack share: at least 40% of orders.
- Refund plus chargeback rate: below 3%.

## Operations and Consumer Obligations

Before accepting payment:

- Form the selling entity, separate banking/bookkeeping, register required tax accounts, and configure sales-tax collection with professional advice.
- Run trademark/domain checks for the final product name.
- Publish accurate specifications, included items, USB requirements, shipping window, return policy, privacy notice, contact method, and warranty terms.
- The FTC requires a reasonable basis for the advertised ship time; without a stated time the default expectation is 30 days. Delays require consent or prompt cancellation/refund ([FTC Mail, Internet, or Telephone Order Rule](https://www.ftc.gov/business-guidance/resources/business-guide-ftcs-mail-internet-or-telephone-order-merchandise-rule)).
- Because both SKUs exceed $15, make any written warranty available before purchase and structure it correctly ([FTC warranty guide](https://www.ftc.gov/business-guidance/resources/businesspersons-guide-federal-warranty-law)).
- Use accurate origin wording. "Assembled in the USA" and "Made in USA" are not interchangeable; unqualified US-origin claims require the product to be all or virtually all US-made ([FTC Made in USA guidance](https://www.ftc.gov/business-guidance/resources/complying-made-usa-standard)).
- Keep evidence for product, performance, shipping, inventory, creator, and origin claims.

Start US-only. International sales add radio regimes, VAT/duties, consumer withdrawal rules, WEEE/RoHS obligations, localization, and expensive returns. Expand only after domestic economics and reliability are proven.

## Weekly Scorecard

Track one sheet by batch and channel:

- units built, first-pass yield, rework time, and landed COGS
- units available, ordered, shipped on time, delivered, returned, replaced, and refunded
- revenue, payment fees, shipping collected/paid, packaging, acquisition spend, and contribution
- support contacts and minutes per order
- Reel reach, qualified profile visits, store sessions, checkout starts, purchases, and bundle share
- failure category by hardware revision and firmware version

The next production order is approved from this scorecard, not from likes or total followers.
