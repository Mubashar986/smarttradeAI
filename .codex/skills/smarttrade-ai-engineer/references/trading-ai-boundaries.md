# Trading AI Boundaries

## Core Boundary

SmartTradeAI can help translate trader intent into structured specs, explain assumptions, prepare generated artifacts, and support review workflows. It must not imply that generated output is profitable, safe, suitable for live trading, or compliant without evidence from the right downstream process.

## Allowed Claims

Allowed when true:

- "Parsed into StrategySpec."
- "Schema validated."
- "Clarification required."
- "Generated draft MQL5 artifact."
- "Ready for MQL5 engineer review."
- "Compile check attempted."
- "Backtest evidence attached."

## Forbidden Claims Without Evidence

Do not claim:

- Profitable.
- Safe.
- Low risk.
- Prop-firm ready.
- Live ready.
- Backtested.
- Optimized.
- Compiled.
- Production ready.
- SEC/CFTC compliant.

## MQL5-Specific Boundaries

- MQL5 supports Expert Advisors, indicators, scripts, services, and include files.
- Expert Advisors can automate trading and send orders to a trading server, so generated EA behavior is high-impact.
- Strategy Tester evidence is the minimum path before discussing tested strategy behavior.
- A compile or static check is not proof of profitability.
- A backtest is not proof of future performance.
- Optimization can overfit; record assumptions and test ranges.

## AI-Washing Boundary

Do not exaggerate what the AI does. If the system uses AI for parsing or draft generation, say that. Do not imply autonomous investment intelligence, predictive power, or validated trading performance unless the implementation and evidence truly support it.

## User-Facing Risk Language

When output touches live trading or money, keep language sober:

```text
This is a generated draft for review and testing. It is not financial advice, not a profit claim, and not ready for live trading without human review, compile checks, and strategy testing.
```

Use this kind of boundary in handoffs and user-facing messages when the task reaches generated trading code or performance discussion.

## Handoff To MQL5 Engineer

Send the MQL5 engineer:

```text
StrategySpec version:
Target artifact:
Required indicators:
Entry rules:
Exit rules:
Risk rules:
Execution assumptions:
Missing or ambiguous fields:
Disallowed behavior:
Safety flags:
AI-generated draft attached:
Evidence so far:
Review needed:
```

## Source Anchors

- MQL5 Reference on program types and Expert Advisors: https://www.mql5.com/en/docs
- MetaTrader 5 Strategy Tester documentation: https://www.metatrader5.com/en/terminal/help/algotrading/testing
- CFTC forex fraud advisory and risk cautions: https://www.cftc.gov/LearnAndProtect/AdvisoriesAndArticles/fraudadv_forex.html
- SEC AI-washing enforcement example: https://www.sec.gov/newsroom/press-releases/2024-36
