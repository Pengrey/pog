// ═══════════════════════════════════════════════════════════════════════
//  pog – Security Assessment Report Template  (Typst)
//  ─────────────────────────────────────────────────────────────────────
//  A professional, self-contained template consumed by `pog report`.
//  No external images required — uses only built-in Typst features.
//
//  Available sys.inputs (all strings unless noted):
//    findings       – array of dicts (num, title, severity, asset, date,
//                     location, report-content, status, images)
//    date           – report generation date  (YYYY/MM/DD)
//    asset          – asset name
//    from / to      – assessment date range
//    total, critical, high, medium, low, info – finding counts (int)
// ═══════════════════════════════════════════════════════════════════════

// ─── data from pog ───
#let data      = sys.inputs
#let findings  = data.at("findings", default: ())
#let asset     = data.at("asset", default: "Target")
#let from-date = data.at("from", default: "")
#let to-date   = data.at("to", default: "")
#let date      = data.at("date", default: "")
#let total     = int(data.at("total", default: "0"))
#let critical  = int(data.at("critical", default: "0"))
#let high      = int(data.at("high", default: "0"))
#let medium    = int(data.at("medium", default: "0"))
#let low       = int(data.at("low", default: "0"))
#let info      = int(data.at("info", default: "0"))

// ═══════════════════════════════════════════════════════════════════════
//  DESIGN TOKENS
// ═══════════════════════════════════════════════════════════════════════

#let accent      = rgb("#1B2A4A")   // deep navy
#let accent-mid  = rgb("#2C4A7C")   // mid blue
#let accent-lite = rgb("#E8EDF4")   // pale blue-gray
#let text-body   = rgb("#2D2D2D")   // soft black for body
#let text-muted  = rgb("#6B7280")   // gray for secondary text
#let divider     = rgb("#CBD5E1")   // light divider
#let page-bg     = white
#let code-bg     = rgb("#F6F7F9")
#let code-border = rgb("#D1D5DB")

#let sev-color(sev) = {
  let s = lower(sev)
  if s == "critical" { rgb("#7F1D1D") }
  else if s == "high"   { rgb("#9A3412") }
  else if s == "medium" { rgb("#92700C") }
  else if s == "low"    { rgb("#3D6B4F") }
  else if s == "info"   { rgb("#475569") }
  else { text-muted }
}

#let status-color(st) = {
  let s = lower(st)
  if s == "open"          { rgb("#9B2C2C") }
  else if s == "inprogress" or s == "in progress" { rgb("#92700C") }
  else if s == "resolved"   { rgb("#3D6B4F") }
  else if s == "falsepositive" or s == "false positive" { rgb("#6B7280") }
  else { text-muted }
}

// ═══════════════════════════════════════════════════════════════════════
//  PAGE SETUP
// ═══════════════════════════════════════════════════════════════════════

#set page(
  paper: "a4",
  margin: (top: 32mm, bottom: 24mm, left: 28mm, right: 28mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(7.5pt, fill: text-muted)
      grid(
        columns: (1fr, auto),
        align: (left + horizon, right + horizon),
        smallcaps[Security Assessment Report],
        [#asset],
      )
      v(3pt)
      line(length: 100%, stroke: 0.6pt + divider)
    }
  },
  footer: context {
    if counter(page).get().first() > 1 {
      line(length: 100%, stroke: 0.4pt + divider)
      v(3pt)
      set text(7.5pt, fill: text-muted)
      grid(
        columns: (1fr, auto, 1fr),
        align: (left, center, right),
        [CONFIDENTIAL],
        [#counter(page).display("1 / 1", both: true)],
        [#date],
      )
    }
  },
)

#set text(font: "Libertinus Serif", size: 10.5pt, fill: text-body)
#set par(justify: true, leading: 0.72em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => {
  v(4pt)
  text(16pt, weight: "bold", fill: accent)[#it]
  v(2pt)
  line(length: 100%, stroke: 1.6pt + accent)
  v(8pt)
}
#show heading.where(level: 2): it => {
  v(6pt)
  text(13pt, weight: "bold", fill: accent-mid)[#it]
  v(4pt)
}

// ─── code styling ───
#show raw.where(block: true): it => block(
  width: 100%,
  fill: code-bg,
  stroke: 0.5pt + code-border,
  inset: 10pt,
  it,
)
#show raw.where(block: false): it => {
  highlight(fill: rgb("#ECEEF1"), text(size: 9.5pt, fill: rgb("#374151"), it))
}

// ═══════════════════════════════════════════════════════════════════════
//  REUSABLE COMPONENTS
// ═══════════════════════════════════════════════════════════════════════

// ── severity badge ──
#let sev-badge(sev) = {
  box(
    fill: sev-color(sev),
    inset: (x: 7pt, y: 3pt),
    text(white, weight: "bold", size: 8pt, font: "Libertinus Sans",
         tracking: 0.5pt, upper(sev)),
  )
}

// ── status badge ──
#let status-badge(st) = {
  box(
    fill: status-color(st).lighten(92%),
    stroke: 0.5pt + status-color(st),
    inset: (x: 6pt, y: 2.5pt),
    text(status-color(st), weight: "bold", size: 7.5pt,
         font: "Libertinus Sans", upper(st)),
  )
}

// ── key-value row ──
#let kv(key, value) = {
  grid(
    columns: (90pt, 1fr),
    text(weight: "bold", size: 9.5pt, fill: text-muted)[#key],
    text(size: 9.5pt)[#value],
  )
}

// ── horizontal severity bar chart ──
#let sev-bars() = {
  let counts = (
    ("Critical", critical, sev-color("critical")),
    ("High",     high,     sev-color("high")),
    ("Medium",   medium,   sev-color("medium")),
    ("Low",      low,      sev-color("low")),
    ("Info",     info,     sev-color("info")),
  )
  let max-count = calc.max(1, critical, high, medium, low, info)

  for (label, count, color) in counts {
    let pct = calc.min(100, calc.round(count / max-count * 100))
    grid(
      columns: (60pt, 28pt, 1fr),
      align: (right + horizon, center + horizon, left + horizon),
      gutter: 6pt,
      text(8.5pt, fill: text-muted, weight: "bold")[#label],
      text(9pt, weight: "bold", fill: color)[#count],
      {
        if count > 0 {
          box(
            width: calc.max(4pt, pct * 1pt),
            height: 10pt,
            fill: color,
          )
        }
      },
    )
    v(3pt)
  }
}

// ── stat card ──
#let stat-card(label, value, color) = {
  block(
    width: 100%,
    fill: color.lighten(95%),
    stroke: (left: 2pt + color, rest: 0.4pt + divider),
    inset: (x: 12pt, y: 8pt),
  )[
    #text(18pt, weight: "bold", fill: color)[#value]
    #v(1pt)
    #text(8pt, fill: text-muted, weight: "bold", tracking: 0.3pt)[#upper(label)]
  ]
}

// ═══════════════════════════════════════════════════════════════════════
//  COVER PAGE
// ═══════════════════════════════════════════════════════════════════════

// Full-width accent band at top
#place(top + left, dx: -28mm, dy: -32mm,
  rect(width: 210mm, height: 12mm, fill: accent),
)

#v(30mm)

// Title block
#align(center)[
  #block(width: 80%)[
    #text(11pt, fill: text-muted, weight: "bold", tracking: 1pt,
          font: "Libertinus Sans")[#upper[Security Assessment]]
    #v(6pt)
    #line(length: 40%, stroke: 1.5pt + accent)
    #v(10pt)
    #text(32pt, weight: "bold", fill: accent)[Report]
    #v(14pt)
    #text(18pt, fill: accent-mid)[#asset]
  ]
]

#v(1fr)

// Info grid
#block(width: 100%, inset: (x: 16pt))[
  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    {
      kv("Prepared for:", asset)
      v(6pt)
      kv("Report Date:", date)
    },
    {
      kv("Period From:", from-date)
      v(6pt)
      kv("Period To:", to-date)
    },
  )
]

#v(12mm)

// Classification bar at bottom
#place(bottom + left, dx: -28mm, dy: 24mm,
  block(width: 210mm, fill: accent, inset: (x: 28mm, y: 6pt))[
    #text(8pt, fill: white, weight: "bold", tracking: 0.8pt,
          font: "Libertinus Sans")[
      #upper[Confidential — For authorised recipients only]
    ]
  ],
)

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  TABLE OF CONTENTS
// ═══════════════════════════════════════════════════════════════════════

= Table of Contents

#v(4pt)
#outline(title: none, indent: 1.5em)

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  1. EXECUTIVE SUMMARY
// ═══════════════════════════════════════════════════════════════════════

= Executive Summary

This report presents the results of a security assessment performed
against *#asset* during the period *#from-date* to *#to-date*.  The
assessment identified a total of *#total* finding(s) across five
severity tiers.

#v(8pt)

// ── stat cards row ──
#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 10pt,
  stat-card("Total Findings", str(total), accent),
  stat-card("Critical / High", str(critical + high), sev-color("critical")),
  stat-card("Medium / Low", str(medium + low), sev-color("medium")),
)

#v(12pt)

// ── alert banner ──
#if critical > 0 [
  #block(
    width: 100%,
    fill: sev-color("critical").lighten(95%),
    stroke: (left: 3pt + sev-color("critical"), rest: 0.4pt + divider),
    inset: 12pt,
  )[
    #text(weight: "bold", fill: sev-color("critical"))[CRITICAL FINDINGS IDENTIFIED]
    #v(4pt)
    #text(size: 10pt)[
      *#critical* critical-severity finding(s) were discovered that may
      lead to full system compromise, data breach, or service disruption.
      These require *immediate remediation*.
    ]
  ]
  #v(6pt)
]

#if high > 0 and critical == 0 [
  #block(
    width: 100%,
    fill: sev-color("high").lighten(95%),
    stroke: (left: 3pt + sev-color("high"), rest: 0.4pt + divider),
    inset: 12pt,
  )[
    #text(weight: "bold", fill: sev-color("high"))[HIGH-SEVERITY FINDINGS]
    #v(4pt)
    #text(size: 10pt)[
      *#high* high-severity finding(s) pose significant risk and should
      be addressed in the short term.
    ]
  ]
  #v(6pt)
]

== Severity Distribution

#grid(
  columns: (55%, 1fr),
  gutter: 16pt,
  // left: bar chart
  sev-bars(),
  // right: summary table
  table(
    columns: (1fr, auto),
    inset: 7pt,
    stroke: 0.4pt + divider,
    fill: (_, row) => if row == 0 { accent } else if calc.rem(row, 2) == 1 { accent-lite } else { white },
    table.header(
      text(white, weight: "bold", size: 8.5pt)[Severity],
      text(white, weight: "bold", size: 8.5pt)[Count],
    ),
    [Critical], [#critical],
    [High],     [#high],
    [Medium],   [#medium],
    [Low],      [#low],
    [Info],     [#info],
  ),
)

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  2. SCOPE & METHODOLOGY
// ═══════════════════════════════════════════════════════════════════════

= Scope and Methodology

== Scope

The assessment targeted the asset identified as *#asset*.  Testing
was conducted during the window *#from-date* through *#to-date* and
encompassed both automated scanning and manual analysis techniques,
including but not limited to:

- Automated vulnerability scanning and enumeration
- Manual code review and configuration analysis
- Authentication and authorisation testing
- Input validation and injection testing
- Session management and cryptographic controls review

== Severity Classification

Findings are classified using a five-tier severity model aligned with
industry-standard frameworks.  The table below outlines each tier
together with the expected remediation response.

#v(6pt)

#table(
  columns: (auto, 1fr, auto),
  inset: 8pt,
  stroke: 0.4pt + divider,
  fill: (_, row) => if row == 0 { accent } else if calc.rem(row, 2) == 1 { accent-lite } else { white },
  table.header(
    text(white, weight: "bold", size: 8.5pt)[Severity],
    text(white, weight: "bold", size: 8.5pt)[Description],
    text(white, weight: "bold", size: 8.5pt)[Response],
  ),
  [#sev-badge("Critical")], [Trivial exploitation leading to full system compromise, data breach, or service disruption.], [Immediate],
  [#sev-badge("High")],     [Likely exploitation with significant impact to security posture.], [Short-term],
  [#sev-badge("Medium")],   [Exploitation requires specific conditions but could result in meaningful impact.], [Planned],
  [#sev-badge("Low")],      [Limited impact; exploitation is difficult or requires significant prerequisites.], [Routine],
  [#sev-badge("Info")],     [Informational observation or defence-in-depth recommendation.], [Advisory],
)

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  3. FINDINGS OVERVIEW
// ═══════════════════════════════════════════════════════════════════════

= Findings Overview

The following table provides a high-level summary of all findings
identified during the engagement.

#v(8pt)

#table(
  columns: (auto, auto, 1fr, auto, auto),
  inset: 8pt,
  stroke: 0.4pt + divider,
  fill: (_, row) => if row == 0 { accent } else if calc.rem(row, 2) == 1 { accent-lite } else { white },
  table.header(
    text(white, weight: "bold", size: 8.5pt)[ID],
    text(white, weight: "bold", size: 8.5pt)[Severity],
    text(white, weight: "bold", size: 8.5pt)[Title],
    text(white, weight: "bold", size: 8.5pt)[Status],
    text(white, weight: "bold", size: 8.5pt)[Date],
  ),
  ..findings.map(f => (
    text(weight: "bold")[V#f.at("num")],
    sev-badge(f.at("severity")),
    f.at("title"),
    status-badge(f.at("status")),
    text(size: 9pt, fill: text-muted)[#f.at("date")],
  )).flatten(),
)

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  4. DETAILED FINDINGS
// ═══════════════════════════════════════════════════════════════════════

= Detailed Findings

#for f in findings {
  pagebreak(weak: true)

  // ── finding header ──
  block(
    width: 100%,
    stroke: (bottom: 0.8pt + divider),
    inset: (bottom: 6pt),
  )[
    #grid(
      columns: (1fr, auto),
      align: (left + horizon, right + horizon),
      [
        #text(12pt, weight: "bold", fill: accent)[V#f.at("num") #sym.dash.en #f.at("title")]
      ],
      sev-badge(f.at("severity")),
    )
  ]

  v(6pt)

  // ── metadata ──
  {
    set text(size: 9pt, fill: text-muted)
    grid(
      columns: (70pt, 1fr),
      row-gutter: 4pt,
      text(weight: "bold")[Location:], [#f.at("location")],
      text(weight: "bold")[Asset:],    [#f.at("asset")],
      text(weight: "bold")[Status:],   status-badge(f.at("status")),
      text(weight: "bold")[Date:],     [#f.at("date")],
    )
  }

  v(8pt)

  // ── report content (converted from markdown to Typst markup) ──
  block(width: 100%, inset: (x: 2pt))[
    #eval(f.at("report-content"), mode: "markup")
  ]

  v(16pt)
}

#pagebreak()

// ═══════════════════════════════════════════════════════════════════════
//  5. CONCLUSION & RECOMMENDATIONS
// ═══════════════════════════════════════════════════════════════════════

= Conclusion and Recommendations

This assessment documented *#total* finding(s) across the target
*#asset*.  The severity breakdown is summarised below.

#v(8pt)

#grid(
  columns: (50%, 1fr),
  gutter: 16pt,
  // severity table
  table(
    columns: (1fr, auto),
    inset: 7pt,
    stroke: 0.4pt + divider,
    fill: (_, row) => if row == 0 { accent } else if calc.rem(row, 2) == 1 { accent-lite } else { white },
    table.header(
      text(white, weight: "bold", size: 8.5pt)[Severity],
      text(white, weight: "bold", size: 8.5pt)[Count],
    ),
    [Critical], [#critical],
    [High],     [#high],
    [Medium],   [#medium],
    [Low],      [#low],
    [Info],     [#info],
  ),
  // recommendations summary
  block(inset: (top: 8pt))[
    #if critical > 0 or high > 0 [
      == Immediate Actions

      Findings rated *Critical* or *High* should be addressed as a
      priority.  A targeted reassessment is recommended following
      remediation to verify effectiveness.
    ] else [
      No critical or high severity findings were identified.
      Nonetheless, all findings should be reviewed and addressed
      according to the organisation's risk management procedures.
    ]

    #v(6pt)

    == Next Steps

    + Prioritise remediation of Critical and High findings.
    + Schedule a verification retest.
    + Review Medium and Low findings during the next development cycle.
    + Incorporate informational items into security hardening standards.
  ],
)

#v(1fr)

// ── confidentiality footer ──
#line(length: 100%, stroke: 0.5pt + divider)
#v(4pt)
#block(width: 100%, inset: (x: 4pt))[
  #set text(size: 8pt, fill: text-muted)
  *Disclaimer* — This report is confidential and intended solely for the
  named recipient.  It reflects the security posture of the target system
  at the time of assessment and does not constitute a guarantee of
  security.  Redistribution without written authorisation is prohibited.
]
