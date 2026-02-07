from email.message import EmailMessage

from examples.fastapi.src.email.core import get_email_client
from examples.fastapi.src.tasks.core import fluxqueue


@fluxqueue.task()
async def send_welcome_email_task(to_email: str):
    async with get_email_client() as email_client:
        message = EmailMessage()
        message["From"] = "test@test.com"
        message["To"] = to_email
        message["Subject"] = "Welcome!"
        message.set_content("Thanks for using FluxQueue")

        await email_client.send_message(message)
