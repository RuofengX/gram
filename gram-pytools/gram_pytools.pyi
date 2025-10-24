def get_mentioned(msg: str) -> tuple[set[str], set[int]] | None:
    """
    输入json字符串格式的tl::enums::Message
    
    函数分析其内容是否包含公群链接,  
    同时对entities进行提取(Mention, mentionName, textUrl),
    如果有则提取用户名或用户ID

    返回两个列表, 分别包含用户名和用户ID
    其中, 用户名是不带@前缀的
    """
    ...
