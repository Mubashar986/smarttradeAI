# SmartTradeAI Concept of Operations

Status: Draft for June MVP
Date: 2026-06-02

## Mission Statement

SmartTradeAI helps a trader convert a clearly described strategy into a
documented MQL5 Expert Advisor draft with risk parameters, basic validation,
and a plain-English explanation.

The system supports software automation. It does not provide financial advice
or guarantee trading performance.

## Primary User

Semi-technical trader:

- Has a strategy idea or existing manual trading rule.
- Understands terms like pair, timeframe, entry, exit, stop loss, and risk.
- Cannot reliably write or debug MQL5.
- Wants transparent source code, not a black-box robot.

## Secondary Users

| User | Use |
| --- | --- |
| Prop-firm trader | Needs risk guard concepts and daily loss constraints expressed in code. |
| Trader with broken AI code | Needs repair, explanation, and MQL4/MQL5 cleanup. |
| Project owner | Uses the system internally to deliver service work faster. |
| Reviewer/employer | Evaluates the engineering process and portfolio value. |

## Operating Modes

| Mode | Description | June Scope |
| --- | --- | --- |
| Internal service mode | Project owner uses SmartTradeAI to fulfill client work. | In scope |
| Demo mode | Show one complete strategy-to-code flow. | In scope |
| Self-service SaaS | Trader uses the app without human review. | Out of scope |
| Live trading mode | System executes trades or manages account. | Out of scope |

## Nominal MVP Flow

1. Trader provides a strategy description.
2. System classifies the request.
3. System extracts strategy fields.
4. System asks one missing detail at a time.
5. System generates MQL5 from controlled templates.
6. System runs basic static validation.
7. System explains generated logic.
8. System saves the strategy artifact.
9. Project owner manually reviews before any client delivery.

## Required Strategy Fields

| Field | Required | Example |
| --- | --- | --- |
| action | Yes | buy, sell, both |
| instrument | Yes | EURUSD, XAUUSD |
| timeframe | Yes | M15, H1 |
| entry condition | Yes | RSI crosses above 30 |
| exit condition | Yes | reverse signal or fixed TP |
| stop loss | Yes | 30 pips |
| risk sizing | Preferred | 1 percent risk |
| daily loss limit | Preferred for prop-firm workflow | stop at 3 percent daily loss |

## Safety Boundary

The product can claim:

- "Generates a documented MQL5 draft."
- "Uses controlled templates."
- "Includes basic static checks."
- "Explains the code in plain English."

The product cannot claim:

- "Profitable."
- "Live ready."
- "Passes prop firm challenges."
- "Guaranteed to match TradingView."
- "Verified trading strategy."

## Success Definition

June MVP succeeds when:

- The demo flow works from strategy text to saved generated code.
- The output is understandable to a trader.
- The limitations are explicit.
- The project owner can explain the system architecture and safety boundary.
- The workflow can support a productized service offer.
