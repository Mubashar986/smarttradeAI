//+------------------------------------------------------------------+
//|                                           {{STRATEGY_NAME}}.mq5  |
//|                      Grid Trading — SmartTradeAI                  |
//+------------------------------------------------------------------+
#property copyright "SmartTradeAI"
#property link      "https://smarttrade.ai"
#property version   "1.00"
#property strict

// ======================== INPUT PARAMETERS ========================
input double LotSize        = 0.01;     // Lot size per grid level
input int    GridSpacingPips = 20;       // Pips between grid levels
input int    GridLevels      = 5;        // Number of grid levels
input int    StopLossPips    = 100;      // Overall stop-loss in pips
input int    TakeProfitPips  = 20;       // Take-profit per level
input int    MagicNumber     = 50001;    // Magic number
// {{PARAMETERS}}

// ======================== GLOBAL VARIABLES ========================
#include <Trade\Trade.mqh>
CTrade trade;
double gridBasePrice;
bool gridInitialized;

int OnInit()
{
    trade.SetExpertMagicNumber(MagicNumber);
    trade.SetDeviationInPoints(10);
    gridInitialized = false;
    Print("{{STRATEGY_NAME}} initialized — Grid: ", GridLevels, " levels @ ", GridSpacingPips, " pips");
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    Print("{{STRATEGY_NAME}} deinitialized");
}

void OnTick()
{
    // Initialize grid base price on first tick
    if(!gridInitialized)
    {
        gridBasePrice = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
        gridInitialized = true;
    }

    // Count existing positions
    int posCount = 0;
    for(int i = PositionsTotal() - 1; i >= 0; i--)
    {
        if(PositionGetTicket(i) > 0 && PositionGetInteger(POSITION_MAGIC) == MagicNumber)
            posCount++;
    }

    // ======================== ENTRY LOGIC ========================
    // {{ENTRY_LOGIC}}

    double currentPrice = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
    double gridStep = GridSpacingPips * PipValue();

    // Place buy orders at grid levels below current price
    if(posCount < GridLevels)
    {
        for(int level = 1; level <= GridLevels - posCount; level++)
        {
            double entryPrice = gridBasePrice - level * gridStep;
            if(currentPrice <= entryPrice + gridStep * 0.1)
            {
                double sl = entryPrice - StopLossPips * PipValue();
                double tp = entryPrice + TakeProfitPips * PipValue();
                trade.Buy(LotSize, _Symbol, currentPrice, sl, tp,
                         StringFormat("Grid Level %d", level));
                break; // One order per tick
            }
        }
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
