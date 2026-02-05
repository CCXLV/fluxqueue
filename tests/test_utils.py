from fluxqueue.utils import get_task_name


def test_task_name():
    def send_email():
        pass

    task_name = get_task_name(send_email)
    assert task_name == "send-email"

    second_task_name = get_task_name(send_email, "send_email_to_users")
    assert second_task_name == "send-email-to-users"
