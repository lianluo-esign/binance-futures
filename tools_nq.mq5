#property copyright "Your Name"
#property link      "https://www.example.com"
#property version   "1.01"
#property strict

// 输入参数
input double LotSize = 1.0;       // 手数
input double TakeProfitPoints = 100; // 止盈点数 (NQ点数，0.25=1点)
input double StopLossPoints = 50;   // 止损点数 (NQ点数，0.25=1点)

// 全局变量
int buttonWidth = 100;
int buttonHeight = 30;
string buyButtonName = "BuyButton";
string sellButtonName = "SellButton";
bool buyButtonPressed = false;
bool sellButtonPressed = false;

// 初始化函数
void OnInit()
{
   // 创建做多按钮
   if (!ObjectCreate(0, buyButtonName, OBJ_BUTTON, 0, 0, 0))
   {
      Print("Failed to create Buy button, error #", GetLastError());
      return;
   }
   ObjectSetInteger(0, buyButtonName, OBJPROP_XDISTANCE, (ChartGetInteger(0, CHART_WIDTH_IN_PIXELS) - 2 * buttonWidth - 10) / 2);
   ObjectSetInteger(0, buyButtonName, OBJPROP_YDISTANCE, 10);
   ObjectSetInteger(0, buyButtonName, OBJPROP_XSIZE, buttonWidth);
   ObjectSetInteger(0, buyButtonName, OBJPROP_YSIZE, buttonHeight);
   ObjectSetString(0, buyButtonName, OBJPROP_TEXT, "Buy");
   ObjectSetInteger(0, buyButtonName, OBJPROP_COLOR, clrWhite);
   ObjectSetInteger(0, buyButtonName, OBJPROP_BGCOLOR, clrBlue);
   ObjectSetInteger(0, buyButtonName, OBJPROP_BORDER_COLOR, clrBlack);
   ObjectSetInteger(0, buyButtonName, OBJPROP_STATE, false); // 确保按钮初始未按下
   ObjectSetInteger(0, buyButtonName, OBJPROP_ZORDER, 10);   // 提高按钮层级

   // 创建做空按钮
   if (!ObjectCreate(0, sellButtonName, OBJ_BUTTON, 0, 0, 0))
   {
      Print("Failed to create Sell button, error #", GetLastError());
      return;
   }
   ObjectSetInteger(0, sellButtonName, OBJPROP_XDISTANCE, (ChartGetInteger(0, CHART_WIDTH_IN_PIXELS) - 2 * buttonWidth - 10) / 2 + buttonWidth + 10);
   ObjectSetInteger(0, sellButtonName, OBJPROP_YDISTANCE, 10);
   ObjectSetInteger(0, sellButtonName, OBJPROP_XSIZE, buttonWidth);
   ObjectSetInteger(0, sellButtonName, OBJPROP_YSIZE, buttonHeight);
   ObjectSetString(0, sellButtonName, OBJPROP_TEXT, "Sell");
   ObjectSetInteger(0, sellButtonName, OBJPROP_COLOR, clrWhite);
   ObjectSetInteger(0, sellButtonName, OBJPROP_BGCOLOR, clrRed);
   ObjectSetInteger(0, sellButtonName, OBJPROP_BORDER_COLOR, clrBlack);
   ObjectSetInteger(0, sellButtonName, OBJPROP_STATE, false); // 确保按钮初始未按下
   ObjectSetInteger(0, sellButtonName, OBJPROP_ZORDER, 10);   // 提高按钮层级

   ChartRedraw();
}

// 反初始化函数
void OnDeinit(const int reason)
{
   ObjectDelete(0, buyButtonName);
   ObjectDelete(0, sellButtonName);
}

// 图表事件处理函数
void OnChartEvent(const int id, const long &lparam, const double &dparam, const string &sparam)
{
   if (id == CHARTEVENT_OBJECT_CLICK)
   {
      if (sparam == buyButtonName)
      {
         buyButtonPressed = ObjectGetInteger(0, buyButtonName, OBJPROP_STATE);
         if (buyButtonPressed)
         {
            ObjectSetInteger(0, buyButtonName, OBJPROP_BGCOLOR, clrDarkBlue); // 按下时颜色变暗
            ObjectSetInteger(0, buyButtonName, OBJPROP_YDISTANCE, 12);        // 轻微下移
            ChartRedraw();
            Sleep(100); // 短暂延迟以显示按下效果
            OpenTrade(ORDER_TYPE_BUY);
            ObjectSetInteger(0, buyButtonName, OBJPROP_BGCOLOR, clrBlue);     // 恢复颜色
            ObjectSetInteger(0, buyButtonName, OBJPROP_YDISTANCE, 10);        // 恢复位置
            ObjectSetInteger(0, buyButtonName, OBJPROP_STATE, false);         // 重置状态
            ChartRedraw();
         }
      }
      else if (sparam == sellButtonName)
      {
         sellButtonPressed = ObjectGetInteger(0, sellButtonName, OBJPROP_STATE);
         if (sellButtonPressed)
         {
            ObjectSetInteger(0, sellButtonName, OBJPROP_BGCOLOR, clrDarkRed); // 按下时颜色变暗
            ObjectSetInteger(0, sellButtonName, OBJPROP_YDISTANCE, 12);       // 轻微下移（修复拼写错误）
            ChartRedraw();
            Sleep(100); // 短暂延迟以显示按下效果
            OpenTrade(ORDER_TYPE_SELL);
            ObjectSetInteger(0, sellButtonName, OBJPROP_BGCOLOR, clrRed);     // 恢复颜色
            ObjectSetInteger(0, sellButtonName, OBJPROP_YDISTANCE, 10);       // 恢复位置
            ObjectSetInteger(0, sellButtonName, OBJPROP_STATE, false);        // 重置状态
            ChartRedraw();
         }
      }
   }
}

// 开单函数
void OpenTrade(ENUM_ORDER_TYPE orderType)
{
   MqlTradeRequest request = {};
   MqlTradeResult result = {};

   // 获取当前价格
   double price = (orderType == ORDER_TYPE_BUY) ? SymbolInfoDouble(_Symbol, SYMBOL_ASK) : SymbolInfoDouble(_Symbol, SYMBOL_BID);
   if (price == 0)
   {
      Print("Failed to get market price for ", _Symbol);
      MessageBox("Failed to get market price for " + _Symbol, "Trade Error", MB_OK | MB_ICONERROR);
      return;
   }

   // 计算止盈和止损价格 (NQ点数：0.25=1点)
   double point = SymbolInfoDouble(_Symbol, SYMBOL_POINT);
   double tickSize = SymbolInfoDouble(_Symbol, SYMBOL_TRADE_TICK_SIZE); // 0.25 for NQ
   if (tickSize == 0)
   {
      Print("Invalid tick size for ", _Symbol);
      MessageBox("Invalid tick size for " + _Symbol, "Trade Error", MB_OK | MB_ICONERROR);
      return;
   }
   double pointsPerTick = tickSize / point; // 转换为MT5的点数
   double tpPoints = TakeProfitPoints * pointsPerTick;
   double slPoints = StopLossPoints * pointsPerTick;

   double tpPrice = (orderType == ORDER_TYPE_BUY) ? price + tpPoints * point : price - tpPoints * point;
   double slPrice = (orderType == ORDER_TYPE_BUY) ? price - slPoints * point : price + slPoints * point;

   // 填充交易请求
   request.action = TRADE_ACTION_DEAL;
   request.symbol = _Symbol;
   request.volume = LotSize;
   request.type = orderType;
   request.price = NormalizeDouble(price, _Digits);
   request.tp = NormalizeDouble(tpPrice, _Digits);
   request.sl = NormalizeDouble(slPrice, _Digits);
   request.type_filling = GetFillingType(); // 动态获取填充类型
   request.deviation = 10; // 设置价格偏差（点数）

   // 发送交易请求
   if (!OrderSend(request, result))
   {
      Print("OrderSend failed for ", EnumToString(orderType), ", error #", GetLastError(), ", retcode: ", result.retcode);
      MessageBox("Failed to open trade: Error #" + IntegerToString(GetLastError()), "Trade Error", MB_OK | MB_ICONERROR);
   }
   else
   {
      Print("Order sent successfully, ticket #", result.order);
      MessageBox(EnumToString(orderType) + " order placed, ticket #" + IntegerToString(result.order), "Trade Success", MB_OK | MB_ICONINFORMATION);
   }
}

// 获取合适的订单填充类型
ENUM_ORDER_TYPE_FILLING GetFillingType()
{
   uint filling = (uint)SymbolInfoInteger(_Symbol, SYMBOL_FILLING_MODE);
   if (filling & SYMBOL_FILLING_IOC) return ORDER_FILLING_IOC;
   if (filling & SYMBOL_FILLING_FOK) return ORDER_FILLING_FOK;
   return ORDER_FILLING_RETURN; // 默认回退
}