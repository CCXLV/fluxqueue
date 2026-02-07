from contextlib import asynccontextmanager

import aiosmtplib


@asynccontextmanager
async def get_email_client():
    client = aiosmtplib.SMTP(
        hostname="localhost",
        port=2525,
    )
    try:
        await client.connect()
        yield client
    finally:
        if client.is_connected:
            await client.quit()
