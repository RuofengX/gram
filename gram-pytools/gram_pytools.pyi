from typing import Optional


def extract_username(message: str, entities: Optional[str]) -> tuple[set[str], set[int]]:
    """
    提取用户名
    :param message: 消息文本内容, 原始内容
    :param entities: 消息entities的JSON-Lines编码, 支持telethon直接导出
    :return: 返回两个列表, 分别为用户名和用户ID, 用户名是不带@前缀的
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
