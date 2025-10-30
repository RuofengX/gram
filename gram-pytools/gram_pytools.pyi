def extract_username(msg: str) -> tuple[set[str], set[int]]:
    """
    输入json字符串格式的tl::enums::Message
    
    函数分析其内容包含的所有用户名和用户ID进行提取,
    如果有则提取用户名或用户ID

    返回两个列表, 分别包含用户名和用户ID
    用户名是不带@前缀的
    """
    ...


def render_text(text: str, scale: float) -> bytes:
    """
    渲染文本为PNG格式字节串
    调用者可使用`io.BytesIO`将返回值包装后使用`PIL.Image.open`打开

    :param text: 待渲染文本
    :param scale: 字体尺寸，推荐值为72
    :return: PNG格式的字节串
    """
    ...
