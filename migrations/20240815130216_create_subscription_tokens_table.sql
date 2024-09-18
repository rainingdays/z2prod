-- Add migration script here
Create Table subscription_tokens(
    subscription_token Text Not Null,
    subscriber_id uuid Not Null References subscriptions (id),
    Primary Key(subscription_token)
);
