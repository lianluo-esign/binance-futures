from openai import OpenAI

client = OpenAI(api_key="sk-393c3ff93d354526af314be2dd425d11", base_url="https://api.deepseek.com/v1")

response = client.chat.completions.create(
    model="deepseek-reasoner",
    messages=[
        {"role": "system", "content": "You are a helpful assistant"},
        {"role": "user", "content": "请帮我分析出今天金十数据首页的最新新闻，并告诉我当前对黄金价格的影响。"},
    ],
    stream=True
)

print(response.choices[0].message.content)