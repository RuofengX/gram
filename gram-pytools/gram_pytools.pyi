from typing import Optional


def extract_entity(message: str, entity: str) -> Optional[str]:
    """
    提取实体对应的文本切片
    :param message: 消息文本内容, 原始内容
    :param entity: 消息实体的JSON编码, 支持telethon格式
    :return: 返回该entity对应的、在message中的文本内容, 如entity没有文本, 返回None
    """
    ...


def extract_username(message: str, entities: Optional[str]) -> tuple[set[str], set[int]]:
    """
    提取用户名
    :param message: 消息文本内容, 原始内容
    :param entities: 消息entities的JSON-Lines编码, 支持telethon格式
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
