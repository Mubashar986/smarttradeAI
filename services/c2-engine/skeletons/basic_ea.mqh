//+------------------------------------------------------------------+
//|                                           {{STRATEGY_NAME}}.mq5  |
//|                                     SmartTradeAI Code Generator   |
//|                                    https://smarttrade.ai          |
//+------------------------------------------------------------------+
#property copyright "SmartTradeAI"
#property link      "https://smarttrade.ai"
#property version   "1.00"
#property strict

// ======================== INPUT PARAMETERS ========================
input double LotSize       = 0.01;    // Trading lot size
input int    StopLossPips  = 50;      // Stop-loss in pips
input int    TakeProfitPips = 100;    // Take-profit in pips
input int    MagicNumber   = 12345;   // Magic number for order identification
// {{PARAMETERS}}

// ======================== GLOBAL VARIABLES ========================
#include <Trade\Trade.mqh>
CTrade trade;
int OnInit()
{
    trade.SetExpertMagicNumber(MagicNumber);
    trade.SetDeviationInPoints(10);
    trade.SetTypeFilling(ORDER_FILLING_IOC);

    Print("{{STRATEGY_NAME}} initialized successfully");
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    Print("{{STRATEGY_NAME}} deinitialized. Reason: ", reason);
}

void OnTick()
{
    // Only trade if no open positions for this EA
    if(PositionsTotal() > 0)
    {
        for(int i = PositionsTotal() - 1; i >= 0; i--)
        {
            if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
                return; // Already have a position
        }
    }

    // ======================== ENTRY LOGIC ========================
    // {{ENTRY_LOGIC}}

    // ======================== EXIT LOGIC =========================
    // {{EXIT_LOGIC}}
}

// ======================== HELPER FUNCTIONS =======================
double PipValue()
{
    double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
    int digits = (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS);
    return (digits == 3 || digits == 5) ? point * 10 : point;
}

double CalculateSL(bool isBuy)
{
    double price = isBuy ? SymbolInfoDouble(_Symbol, SYMBOL_ASK) : SymbolInfoDouble(_Symbol, SYMBOL_BID);
    double sl = isBuy ? price - StopLossPips * PipValue() : price + StopLossPips * PipValue();
    return NormalizeDouble(sl, (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS));
}

double CalculateTP(bool isBuy)
{
    double price = isBuy ? SymbolInfoDouble(_Symbol, SYMBOL_ASK) : SymbolInfoDouble(_Symbol, SYMBOL_BID);
    double tp = isBuy ? price + TakeProfitPips * PipValue() : price - TakeProfitPips * PipValue();
    return NormalizeDouble(tp, (int)SymbolInfoInteger(_Symbol, SYMBOL_DIGITS));
}
//+------------------------------------------------------------------+
