//+------------------------------------------------------------------+
//|                                           {{STRATEGY_NAME}}.mq5  |
//|                    Breakout Strategy — SmartTradeAI               |
//+------------------------------------------------------------------+
#property copyright "SmartTradeAI"
#property link      "https://smarttrade.ai"
#property version   "1.00"
#property strict

// ======================== INPUT PARAMETERS ========================
input int    LookbackPeriod = 20;       // Bars to look back for highs/lows
input double LotSize        = 0.01;     // Trading lot size
input int    StopLossPips   = 30;       // Stop-loss in pips
input int    TakeProfitPips = 90;       // Take-profit in pips (3:1 R:R)
input int    MagicNumber    = 40001;    // Magic number
// {{PARAMETERS}}

// ======================== GLOBAL VARIABLES ========================
#include <Trade\Trade.mqh>
CTrade trade;

int OnInit()
{
    trade.SetExpertMagicNumber(MagicNumber);
    trade.SetDeviationInPoints(10);
    Print("{{STRATEGY_NAME}} initialized — Lookback: ", LookbackPeriod);
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    Print("{{STRATEGY_NAME}} deinitialized");
}

void OnTick()
{
    // Calculate channel boundaries
    double highestHigh = 0, lowestLow = DBL_MAX;
    for(int i = 1; i <= LookbackPeriod; i++)
    {
        double h = iHigh(_Symbol, PERIOD_CURRENT, i);
        double l = iLow(_Symbol, PERIOD_CURRENT, i);
        if(h > highestHigh) highestHigh = h;
        if(l < lowestLow) lowestLow = l;
    }

    double currentClose = iClose(_Symbol, PERIOD_CURRENT, 0);
    double prevClose = iClose(_Symbol, PERIOD_CURRENT, 1);

    bool hasPosition = false;
    for(int i = PositionsTotal() - 1; i >= 0; i--)
    {
        if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
        {
            hasPosition = true;
            break;
        }
    }

    // ======================== ENTRY LOGIC ========================
    // {{ENTRY_LOGIC}}

    // Breakout above resistance → BUY
    bool breakoutUp = currentClose > highestHigh && prevClose <= highestHigh;
    // Breakout below support → SELL
    bool breakoutDown = currentClose < lowestLow && prevClose >= lowestLow;

    if(breakoutUp && !hasPosition)
    {
        double ask = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
        double sl = ask - StopLossPips * PipValue();
        double tp = ask + TakeProfitPips * PipValue();
        trade.Buy(LotSize, _Symbol, ask, sl, tp, "Resistance Breakout");
    }
    else if(breakoutDown && !hasPosition)
    {
        double bid = SymbolInfoDouble(_Symbol, SYMBOL_BID);
        double sl = bid + StopLossPips * PipValue();
        double tp = bid - TakeProfitPips * PipValue();
        trade.Sell(LotSize, _Symbol, bid, sl, tp, "Support Breakout");
    }

    // ======================== EXIT LOGIC =========================
    // {{EXIT_LOGIC}}
}

double PipValue()
{
    double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
    int digits = (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS);
    return (digits == 3 || digits == 5) ? point * 10 : point;
}
//+------------------------------------------------------------------+
