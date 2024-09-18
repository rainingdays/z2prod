-- Add migration script here
Begin;
    Update subscriptions
        Set status='confirmed'
        Where status Is Null;

        Alter Table subscriptions Alter Column status Set Not Null;
Commit;
