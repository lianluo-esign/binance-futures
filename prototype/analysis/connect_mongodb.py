"""
MongoDB连接模块 - 为高频交易数据分析提供数据访问
"""

import pymongo
from pymongo import MongoClient
from pymongo.errors import ConnectionFailure, OperationFailure, ServerSelectionTimeoutError
import logging
import time
from typing import Dict, List, Optional, Union, Any, Tuple
from datetime import datetime, timedelta

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("mongodb_connector")

class MongoDBConnector:
    """MongoDB连接器 - 提供高频交易数据访问接口"""
    
    def __init__(self, 
                 host: str = 'localhost', 
                 port: int = 27017, 
                 db_name: str = 'hft_data',
                 username: Optional[str] = None,
                 password: Optional[str] = None,
                 max_retries: int = 3,
                 retry_delay: int = 2):
        """
        初始化MongoDB连接器
        
        参数:
            host: MongoDB服务器地址
            port: MongoDB服务器端口
            db_name: 数据库名称
            username: 用户名 (可选)
            password: 密码 (可选)
            max_retries: 连接重试最大次数
            retry_delay: 重试间隔(秒)
        """
        self.host = host
        self.port = port
        self.db_name = db_name
        self.username = username
        self.password = password
        self.max_retries = max_retries
        self.retry_delay = retry_delay
        
        self.client = None
        self.db = None
        self._connect()
    
    def _connect(self) -> None:
        """建立MongoDB连接"""
        retry_count = 0
        
        while retry_count < self.max_retries:
            try:
                # 构建连接URI
                if self.username and self.password:
                    uri = f"mongodb://{self.username}:{self.password}@{self.host}:{self.port}/{self.db_name}"
                else:
                    uri = f"mongodb://{self.host}:{self.port}/{self.db_name}"
                
                # 建立连接
                self.client = MongoClient(uri, serverSelectionTimeoutMS=5000)
                
                # 验证连接
                self.client.admin.command('ping')
                
                # 获取数据库引用
                self.db = self.client[self.db_name]
                
                logger.info(f"成功连接到MongoDB: {self.host}:{self.port}/{self.db_name}")
                return
                
            except (ConnectionFailure, ServerSelectionTimeoutError) as e:
                retry_count += 1
                logger.warning(f"连接MongoDB失败 (尝试 {retry_count}/{self.max_retries}): {str(e)}")
                
                if retry_count >= self.max_retries:
                    logger.error(f"无法连接到MongoDB: {str(e)}")
                    raise ConnectionError(f"无法连接到MongoDB: {str(e)}")
                
                time.sleep(self.retry_delay)
    
    def get_collection(self, collection_name: str) -> pymongo.collection.Collection:
        """
        获取指定的集合

        参数:
            collection_name: 集合名称

        返回:
            pymongo.collection.Collection: 集合对象
        """
        if self.db is None:
            self._connect()
        return self.db[collection_name]
    
    def query_trades(self, 
                     symbol: str, 
                     start_time: Optional[datetime] = None,
                     end_time: Optional[datetime] = None,
                     limit: Optional[int] = None) -> List[Dict]:
        """
        查询交易数据
        
        参数:
            symbol: 交易对符号 (例如 "btcusdt")
            start_time: 开始时间
            end_time: 结束时间
            limit: 限制返回记录数量
            
        返回:
            List[Dict]: 交易记录列表
        """
        collection = self.get_collection(f"trades_history_{symbol}")
        
        # 构建查询条件
        query = {}
        if start_time or end_time:
            time_query = {}
            if start_time:
                time_query["$gte"] = int(start_time.timestamp() * 1000)
            if end_time:
                time_query["$lte"] = int(end_time.timestamp() * 1000)
            query["ts"] = time_query
        
        # 执行查询
        cursor = collection.find(query)
        
        # 应用排序和限制
        cursor = cursor.sort("ts", pymongo.ASCENDING)
        if limit:
            cursor = cursor.limit(limit)
        
        return list(cursor)
    
    def query_orderbook(self, 
                        symbol: str, 
                        start_time: Optional[datetime] = None,
                        end_time: Optional[datetime] = None,
                        limit: Optional[int] = None) -> List[Dict]:
        """
        查询订单簿数据
        
        参数:
            symbol: 交易对符号 (例如 "btcusdt")
            start_time: 开始时间
            end_time: 结束时间
            limit: 限制返回记录数量
            
        返回:
            List[Dict]: 订单簿记录列表
        """
        collection = self.get_collection(f"depth_diff_{symbol}")
        
        # 构建查询条件
        query = {}
        if start_time or end_time:
            time_query = {}
            if start_time:
                time_query["$gte"] = int(start_time.timestamp() * 1000)
            if end_time:
                time_query["$lte"] = int(end_time.timestamp() * 1000)
            query["ts"] = time_query
        
        # 执行查询
        cursor = collection.find(query)
        
        # 应用排序和限制
        cursor = cursor.sort("ts", pymongo.ASCENDING)
        if limit:
            cursor = cursor.limit(limit)
        
        return list(cursor)
    
    def query_footprint(self, 
                        symbol: str, 
                        start_time: Optional[datetime] = None,
                        end_time: Optional[datetime] = None) -> List[Dict]:
        """
        查询Footprint数据
        
        参数:
            symbol: 交易对符号 (例如 "btcusdt")
            start_time: 开始时间
            end_time: 结束时间
            
        返回:
            List[Dict]: Footprint记录列表
        """
        collection = self.get_collection(f"footprint_history_{symbol}")
        
        # 构建查询条件
        query = {"symbol": symbol}
        if start_time or end_time:
            time_query = {}
            if start_time:
                time_query["$gte"] = int(start_time.timestamp() * 1000)
            if end_time:
                time_query["$lte"] = int(end_time.timestamp() * 1000)
            query["timestamp"] = time_query
        
        # 执行查询
        cursor = collection.find(query).sort("timestamp", pymongo.ASCENDING)
        
        return list(cursor)
    
    def get_collection_stats(self, collection_name: str) -> Dict:
        """
        获取集合统计信息
        
        参数:
            collection_name: 集合名称
            
        返回:
            Dict: 集合统计信息
        """
        return self.db.command("collStats", collection_name)
    
    def list_collections(self) -> List[str]:
        """
        列出所有集合名称
        
        返回:
            List[str]: 集合名称列表
        """
        return self.db.list_collection_names()
    
    def get_time_range(self, collection_name: str) -> Tuple[datetime, datetime]:
        """
        获取集合中数据的时间范围
        
        参数:
            collection_name: 集合名称
            
        返回:
            Tuple[datetime, datetime]: (最早时间, 最晚时间)
        """
        collection = self.get_collection(collection_name)
        
        # 查找最早和最晚的记录
        earliest = collection.find_one({}, sort=[("ts", pymongo.ASCENDING)])
        latest = collection.find_one({}, sort=[("ts", pymongo.DESCENDING)])
        
        if not earliest or not latest:
            return (datetime.now(), datetime.now())
        
        # 转换时间戳为datetime对象
        earliest_time = datetime.fromtimestamp(earliest["ts"] / 1000)
        latest_time = datetime.fromtimestamp(latest["ts"] / 1000)
        
        return (earliest_time, latest_time)
    
    def close(self) -> None:
        """关闭MongoDB连接"""
        if self.client:
            self.client.close()
            logger.info("MongoDB连接已关闭")
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()


# 使用示例
if __name__ == "__main__":
    # 连接MongoDB
    try:
        mongo = MongoDBConnector(
            host='localhost',
            port=27017,
            db_name='crypto_data'
        )
        
        # 列出所有集合
        collections = mongo.list_collections()
        print(f"数据库中的集合: {collections}")
        
        # 查询最近1小时的BTC交易数据
        end_time = datetime.now()
        start_time = end_time - timedelta(hours=1)
        
        trades = mongo.query_trades(
            symbol="btcusdt",
            start_time=start_time,
            end_time=end_time,
            limit=10
        )
        
        print(f"最近交易数据示例 (共{len(trades)}条):")
        for trade in trades[:5]:
            print(f"  时间: {datetime.fromtimestamp(trade['ts']/1000)}, 价格: {trade.get('price')}, 数量: {trade.get('qty')}")
        
    except Exception as e:
        print(f"错误: {str(e)}")
    finally:
        # 关闭连接
        if 'mongo' in locals():
            mongo.close()