"""
测试MongoDB查询性能
比较使用ObjectId vs T字段进行时间范围查询的性能差异
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import time
import logging
from datetime import datetime, timedelta
from connect_mongodb import MongoDBConnector
from bson import ObjectId

# 配置日志
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

def test_objectid_query(collection, start_time, end_time):
    """测试使用ObjectId的查询性能"""
    logger.info("测试ObjectId查询...")
    
    # 使用ObjectId进行时间查询
    start_objectid = ObjectId.from_datetime(start_time)
    end_objectid = ObjectId.from_datetime(end_time)
    
    query = {
        "_id": {
            "$gte": start_objectid,
            "$lte": end_objectid
        }
    }
    
    # 测试查询性能
    start_query_time = time.time()
    
    # 先统计数量
    count_start = time.time()
    total_count = collection.count_documents(query)
    count_time = time.time() - count_start
    
    # 查询数据（限制返回数量以避免内存问题）
    query_start = time.time()
    cursor = collection.find(query).sort("_id", 1).limit(10000)
    results = list(cursor)
    query_time = time.time() - query_start
    
    total_time = time.time() - start_query_time
    
    logger.info(f"ObjectId查询结果:")
    logger.info(f"  查询条件: {query}")
    logger.info(f"  总记录数: {total_count:,}")
    logger.info(f"  返回记录数: {len(results):,}")
    logger.info(f"  统计时间: {count_time:.3f}秒")
    logger.info(f"  查询时间: {query_time:.3f}秒")
    logger.info(f"  总时间: {total_time:.3f}秒")
    
    return {
        'method': 'ObjectId',
        'total_count': total_count,
        'returned_count': len(results),
        'count_time': count_time,
        'query_time': query_time,
        'total_time': total_time
    }

def test_timestamp_query(collection, start_time, end_time):
    """测试使用T字段的查询性能"""
    logger.info("测试T字段查询...")
    
    # 使用T字段进行时间查询
    start_timestamp = int(start_time.timestamp() * 1000)
    end_timestamp = int(end_time.timestamp() * 1000)
    
    query = {
        "T": {
            "$gte": start_timestamp,
            "$lte": end_timestamp
        }
    }
    
    # 测试查询性能
    start_query_time = time.time()
    
    # 先统计数量
    count_start = time.time()
    total_count = collection.count_documents(query)
    count_time = time.time() - count_start
    
    # 查询数据（限制返回数量以避免内存问题）
    query_start = time.time()
    cursor = collection.find(query).sort("T", 1).limit(10000)
    results = list(cursor)
    query_time = time.time() - query_start
    
    total_time = time.time() - start_query_time
    
    logger.info(f"T字段查询结果:")
    logger.info(f"  查询条件: {query}")
    logger.info(f"  总记录数: {total_count:,}")
    logger.info(f"  返回记录数: {len(results):,}")
    logger.info(f"  统计时间: {count_time:.3f}秒")
    logger.info(f"  查询时间: {query_time:.3f}秒")
    logger.info(f"  总时间: {total_time:.3f}秒")
    
    return {
        'method': 'T字段',
        'total_count': total_count,
        'returned_count': len(results),
        'count_time': count_time,
        'query_time': query_time,
        'total_time': total_time
    }

def show_index_info(collection):
    """显示集合的索引信息"""
    logger.info("集合索引信息:")
    
    indexes = list(collection.list_indexes())
    for idx in indexes:
        logger.info(f"  索引: {idx['name']}")
        logger.info(f"    键: {idx['key']}")
        if 'background' in idx:
            logger.info(f"    后台创建: {idx['background']}")
        logger.info("")

def compare_query_performance():
    """比较查询性能"""
    try:
        logger.info("开始查询性能测试...")
        
        # 连接MongoDB
        mongo = MongoDBConnector(
            host='localhost',
            port=27017,
            db_name='crypto_data'
        )
        
        # 获取trades集合
        collection = mongo.get_collection('btcusdt_trades')
        
        # 显示索引信息
        show_index_info(collection)
        
        # 设置测试时间范围（最近6小时）
        end_time = datetime.now()
        start_time = end_time - timedelta(hours=6)
        
        logger.info(f"测试时间范围: {start_time} 到 {end_time}")
        
        # 测试ObjectId查询
        objectid_result = test_objectid_query(collection, start_time, end_time)
        
        print("\n" + "="*50)
        
        # 测试T字段查询
        timestamp_result = test_timestamp_query(collection, start_time, end_time)
        
        # 比较结果
        print("\n" + "="*60)
        print("查询性能比较结果")
        print("="*60)
        
        print(f"{'方法':<10} {'总记录数':<12} {'统计时间':<10} {'查询时间':<10} {'总时间':<10}")
        print("-" * 60)
        
        for result in [objectid_result, timestamp_result]:
            print(f"{result['method']:<10} {result['total_count']:<12,} "
                  f"{result['count_time']:<10.3f} {result['query_time']:<10.3f} "
                  f"{result['total_time']:<10.3f}")
        
        # 性能分析
        print("\n性能分析:")
        if objectid_result['total_time'] < timestamp_result['total_time']:
            speedup = timestamp_result['total_time'] / objectid_result['total_time']
            print(f"ObjectId查询比T字段查询快 {speedup:.2f} 倍")
        else:
            speedup = objectid_result['total_time'] / timestamp_result['total_time']
            print(f"T字段查询比ObjectId查询快 {speedup:.2f} 倍")
        
        # 推荐
        print("\n推荐:")
        if objectid_result['total_count'] == timestamp_result['total_count']:
            print("✅ 两种查询方式返回相同数量的记录，数据一致性良好")
            if objectid_result['total_time'] < timestamp_result['total_time']:
                print("🚀 推荐使用ObjectId查询，性能更好且无需额外索引")
            else:
                print("📊 推荐使用T字段查询，但需要确保T字段有索引")
        else:
            print("⚠️  两种查询方式返回不同数量的记录，可能存在数据不一致")
            print("   建议检查数据质量和时间字段的准确性")
        
    except Exception as e:
        logger.error(f"性能测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if 'mongo' in locals():
            mongo.close()

def test_data_consistency():
    """测试数据一致性"""
    try:
        logger.info("测试数据一致性...")
        
        mongo = MongoDBConnector(
            host='localhost',
            port=27017,
            db_name='crypto_data'
        )
        
        collection = mongo.get_collection('btcusdt_trades')
        
        # 获取一些样本数据来比较ObjectId时间戳和T字段
        sample_docs = list(collection.find().limit(100))
        
        print("\n" + "="*60)
        print("数据一致性检查")
        print("="*60)
        
        inconsistent_count = 0
        for doc in sample_docs:
            # 从ObjectId提取时间戳
            objectid_time = doc['_id'].generation_time
            
            # 从T字段获取时间戳
            if 'T' in doc:
                t_field_time = datetime.fromtimestamp(doc['T'] / 1000)
                
                # 比较时间差（允许一定误差）
                time_diff = abs((objectid_time - t_field_time).total_seconds())
                
                if time_diff > 60:  # 如果时间差超过60秒
                    inconsistent_count += 1
                    if inconsistent_count <= 5:  # 只显示前5个不一致的例子
                        print(f"时间不一致: ObjectId={objectid_time}, T字段={t_field_time}, 差异={time_diff:.1f}秒")
        
        consistency_rate = (len(sample_docs) - inconsistent_count) / len(sample_docs) * 100
        print(f"\n数据一致性: {consistency_rate:.1f}% ({len(sample_docs) - inconsistent_count}/{len(sample_docs)})")
        
        if consistency_rate > 95:
            print("✅ 数据一致性良好，可以安全使用ObjectId进行时间查询")
        else:
            print("⚠️  数据一致性较差，建议使用T字段进行时间查询")
            
    except Exception as e:
        logger.error(f"一致性测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if 'mongo' in locals():
            mongo.close()

def main():
    """主函数"""
    print("MongoDB查询性能测试")
    print("="*60)
    
    # 测试查询性能
    compare_query_performance()
    
    print("\n" + "="*60)
    
    # 测试数据一致性
    test_data_consistency()

if __name__ == "__main__":
    main()
