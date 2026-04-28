//+------------------------------------------------------------------+
//|                                           {{STRATEGY_NAME}}.mq5  |
//|                        SMA Crossover — SmartTradeAI               |
//+------------------------------------------------------------------+
#property copyright "SmartTradeAI"
#property link      "https://smarttrade.ai"
#property version   "1.00"
#property strict

// ======================== INPUT PARAMETERS ========================
input int    FastMAPeriod   = 50;       // Fast SMA period
input int    SlowMAPeriod   = 200;      // Slow SMA period
input double LotSize        = 0.01;     // Trading lot size
input int    StopLossPips   = 50;       // Stop-loss in pips
input int    TakeProfitPips = 100;      // Take-profit in pips
input int    MagicNumber    = 20001;    // Magic number
// {{PARAMETERS}}

// ======================== GLOBAL VARIABLES ========================
#include <Trade\Trade.mqh>
CTrade trade;
int handleFastMA, handleSlowMA;

int OnInit()
{
    trade.SetExpertMagicNumber(MagicNumber);
    trade.SetDeviationInPoints(10);

    handleFastMA = iMA(_Symbol, PERIOD_CURRENT, FastMAPeriod, 0, MODE_SMA, PRICE_CLOSE);
    handleSlowMA = iMA(_Symbol, PERIOD_CURRENT, SlowMAPeriod, 0, MODE_SMA, PRICE_CLOSE);

    if(handleFastMA == INVALID_HANDLE || handleSlowMA == INVALID_HANDLE)
    {
        Print("ERROR: Failed to create MA indicators");
        return(INIT_FAILED);
    }

    Print("{{STRATEGY_NAME}} initialized — Fast MA: ", FastMAPeriod, " Slow MA: ", SlowMAPeriod);
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    IndicatorRelease(handleFastMA);
    IndicatorRelease(handleSlowMA);
    Print("{{STRATEGY_NAME}} deinitialized");
}

void OnTick()
{
    // Get MA values for current and previous bar
    double fastMA[2], slowMA[2];
    if(CopyBuffer(handleFastMA, 0, 0, 2, fastMA) < 2) return;
    if(CopyBuffer(handleSlowMA, 0, 0, 2, slowMA) < 2) return;

    // Check for existing position
    bool hasPosition = false;
    long posType = -1;
    for(int i = PositionsTotal() - 1; i >= 0; i--)
    {
        if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
        {
            hasPosition = true;
            posType = PositionGetInteger(POSITION_TYPE);
            break;
        }
    }

    // ======================== ENTRY LOGIC ========================
    // {{ENTRY_LOGIC}}

    // Golden Cross: Fast MA crosses above Slow MA → BUY
    bool goldenCross = fastMA[1] > slowMA[1] && fastMA[0] <= slowMA[0];
    // Death Cross: Fast MA crosses below Slow MA → SELL
    bool deathCross = fastMA[1] < slowMA[1] && fastMA[0] >= slowMA[0];

    if(goldenCross && !hasPosition)
    {
        double ask = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
        double sl = ask - StopLossPips * PipValue();
        double tp = ask + TakeProfitPips * PipValue();
        trade.Buy(LotSize, _Symbol, ask, sl, tp, "SMA Golden Cross");
    }
    else if(deathCross && !hasPosition)
    {
        double bid = SymbolInfoDouble(_Symbol, SYMBOL_BID);
        double sl = bid + StopLossPips * PipValue();
        double tp = bid - TakeProfitPips * PipValue();
        trade.Sell(LotSize, _Symbol, bid, sl, tp, "SMA Death Cross");
    }

    // ======================== EXIT LOGIC =========================
    // {{EXIT_LOGIC}}

    // Close BUY on Death Cross
    if(hasPosition && posType == POSITION_TYPE_BUY && deathCross)
    {
        for(int i = PositionsTotal() - 1; i >= 0; i--)
        {
            if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
                trade.PositionClose(PositionGetTicket(i));
        }
    }
    // Close SELL on Golden Cross
    if(hasPosition && posType == POSITION_TYPE_SELL && goldenCross)
    {
        for(int i = PositionsTotal() - 1; i >= 0; i--)
        {
            if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
                trade.PositionClose(PositionGetTicket(i));
        }
    }
}

double PipValue()
{
    double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
    int digits = (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS);
    return (digits == 3 || digits == 5) ? point * 10 : point;
}
//+------------------------------------------------------------------+
