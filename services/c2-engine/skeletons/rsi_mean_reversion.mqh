//+------------------------------------------------------------------+
//|                                           {{STRATEGY_NAME}}.mq5  |
//|                    RSI Mean Reversion — SmartTradeAI              |
//+------------------------------------------------------------------+
#property copyright "SmartTradeAI"
#property link      "https://smarttrade.ai"
#property version   "1.00"
#property strict

// ======================== INPUT PARAMETERS ========================
input int    RSIPeriod      = 14;       // RSI period
input double RSIOverBought  = 70.0;     // Overbought level
input double RSIOverSold    = 30.0;     // Oversold level
input double LotSize        = 0.01;     // Trading lot size
input int    StopLossPips   = 40;       // Stop-loss in pips
input int    TakeProfitPips = 80;       // Take-profit in pips
input int    MagicNumber    = 30001;    // Magic number
// {{PARAMETERS}}

// ======================== GLOBAL VARIABLES ========================
#include <Trade\Trade.mqh>
CTrade trade;
int handleRSI;

int OnInit()
{
    trade.SetExpertMagicNumber(MagicNumber);
    trade.SetDeviationInPoints(10);

    handleRSI = iRSI(_Symbol, PERIOD_CURRENT, RSIPeriod, PRICE_CLOSE);
    if(handleRSI == INVALID_HANDLE)
    {
        Print("ERROR: Failed to create RSI indicator");
        return(INIT_FAILED);
    }

    Print("{{STRATEGY_NAME}} initialized — RSI Period: ", RSIPeriod);
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    IndicatorRelease(handleRSI);
}

void OnTick()
{
    double rsi[2];
    if(CopyBuffer(handleRSI, 0, 0, 2, rsi) < 2) return;

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

    // BUY when RSI crosses above oversold level (mean reversion up)
    bool buySignal = rsi[1] > RSIOverSold && rsi[0] <= RSIOverSold;
    // SELL when RSI crosses below overbought level (mean reversion down)
    bool sellSignal = rsi[1] < RSIOverBought && rsi[0] >= RSIOverBought;

    if(buySignal && !hasPosition)
    {
        double ask = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
        double sl = ask - StopLossPips * PipValue();
        double tp = ask + TakeProfitPips * PipValue();
        trade.Buy(LotSize, _Symbol, ask, sl, tp, "RSI Oversold Bounce");
    }
    else if(sellSignal && !hasPosition)
    {
        double bid = SymbolInfoDouble(_Symbol, SYMBOL_BID);
        double sl = bid + StopLossPips * PipValue();
        double tp = bid - TakeProfitPips * PipValue();
        trade.Sell(LotSize, _Symbol, bid, sl, tp, "RSI Overbought Reversal");
    }

    // ======================== EXIT LOGIC =========================
    // {{EXIT_LOGIC}}

    if(hasPosition && posType == POSITION_TYPE_BUY && rsi[1] > RSIOverBought)
    {
        for(int i = PositionsTotal() - 1; i >= 0; i--)
            if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
                trade.PositionClose(PositionGetTicket(i));
    }
    if(hasPosition && posType == POSITION_TYPE_SELL && rsi[1] < RSIOverSold)
    {
        for(int i = PositionsTotal() - 1; i >= 0; i--)
            if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
                trade.PositionClose(PositionGetTicket(i));
    }
}

double PipValue()
{
    double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
    int digits = (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS);
    return (digits == 3 || digits == 5) ? point * 10 : point;
}
//+------------------------------------------------------------------+
