from fastapi import FastAPI

from examples.fastapi.src.tasks import send_welcome_email_task

app = FastAPI()


@app.get("/email/{email}")
async def send_welcome_email(email: str):
    await send_welcome_email_task(email)

    return {"message": "Email was sent"}
