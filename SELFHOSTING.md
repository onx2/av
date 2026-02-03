Self-hosting
============

> Note on persistence vs backups:
>
> SpacetimeDB persists data to disk under its configured root directory (e.g. `--root-dir=/stdb`). That means data should survive service restarts and machine reboots.
>
> Backups are still required for self-hosting because persistence alone does not protect you from VM deletion, disk failure, filesystem corruption, or operator mistakes. This doc includes a **no-downtime** backup workflow using **AWS EBS snapshots**.


This tutorial will guide you through setting up SpacetimeDB on an Ubuntu 24.04 server, securing it with HTTPS using Nginx and Let's Encrypt, and configuring a systemd service to keep it running.

Prerequisites[​](#prerequisites "Direct link to Prerequisites")
---------------------------------------------------------------

*   A fresh Ubuntu 24.04 server (VM or cloud instance of your choice)
*   A domain name (e.g., `example.com`)
*   `sudo` privileges on the server

Step 1: Create a Dedicated User for SpacetimeDB[​](#step-1-create-a-dedicated-user-for-spacetimedb "Direct link to Step 1: Create a Dedicated User for SpacetimeDB")
--------------------------------------------------------------------------------------------------------------------------------------------------------------------

For security purposes, create a dedicated `spacetimedb` user to run SpacetimeDB:

    sudo mkdir /stdb
    sudo useradd --system spacetimedb
    sudo chown -R spacetimedb:spacetimedb /stdb

Install SpacetimeDB as the new user:

    sudo -u spacetimedb bash -c 'curl -sSf https://install.spacetimedb.com | sh -s -- --root-dir /stdb --yes'

Step 2: Create a Systemd Service for SpacetimeDB[​](#step-2-create-a-systemd-service-for-spacetimedb "Direct link to Step 2: Create a Systemd Service for SpacetimeDB")
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------

To ensure SpacetimeDB runs on startup, create a systemd service file:

    sudo nano /etc/systemd/system/spacetimedb.service

Add the following content:

    [Unit]
    Description=SpacetimeDB Server
    After=network.target
    
    [Service]
    ExecStart=/stdb/spacetime --root-dir=/stdb start --listen-addr='127.0.0.1:3000'
    Restart=always
    User=spacetimedb
    WorkingDirectory=/stdb
    
    [Install]
    WantedBy=multi-user.target

Enable and start the service:

    sudo systemctl enable spacetimedb
    sudo systemctl start spacetimedb

Check the status:

    sudo systemctl status spacetimedb

Step 3: Install and Configure Nginx[​](#step-3-install-and-configure-nginx "Direct link to Step 3: Install and Configure Nginx")
--------------------------------------------------------------------------------------------------------------------------------

### Install Nginx[​](#install-nginx "Direct link to Install Nginx")

    sudo apt update
    sudo apt install nginx -y

### Configure Nginx Reverse Proxy[​](#configure-nginx-reverse-proxy "Direct link to Configure Nginx Reverse Proxy")

Create a new Nginx configuration file:

    sudo nano /etc/nginx/sites-available/spacetimedb

Add the following configuration, remember to change `example.com` to your own domain:

    server {
        listen 80;
        server_name example.com;
    
        #########################################
        # By default SpacetimeDB is completely open so that anyone can publish to it. If you want to block
        # users from creating new databases you should keep this section commented out. Otherwise, if you
        # want to open it up (probably for dev environments) then you can uncomment this section and then
        # also comment out the location / section below.
        #########################################
        # location / {
        #     proxy_pass http://localhost:3000;
        #     proxy_http_version 1.1;
        #     proxy_set_header Upgrade $http_upgrade;
        #     proxy_set_header Connection "Upgrade";
        #     proxy_set_header Host $host;
        # }
    
        # Anyone can subscribe to any database.
        # Note: This is the only section *required* for the websocket to function properly. Clients will
        # be able to create identities, call reducers, and subscribe to tables through this websocket.
        location ~ ^/v1/database/[^/]+/subscribe$ {
            proxy_pass http://localhost:3000;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_set_header Host $host;
        }
    
        # Uncomment this section to allow all HTTP reducer calls
        # location ~ ^/v1/[^/]+/call/[^/]+$ {
        #     proxy_pass http://localhost:3000;
        #     proxy_http_version 1.1;
        #     proxy_set_header Upgrade $http_upgrade;
        #     proxy_set_header Connection "Upgrade";
        #     proxy_set_header Host $host;
        # }
    
        # Uncomment this section to allow all HTTP sql requests
        # location ~ ^/v1/[^/]+/sql$ {
        #     proxy_pass http://localhost:3000;
        #     proxy_http_version 1.1;
        #     proxy_set_header Upgrade $http_upgrade;
        #     proxy_set_header Connection "Upgrade";
        #     proxy_set_header Host $host;
        # }
    
        # NOTE: This is required for the typescript sdk to function, it is optional
        # for the rust and the C# SDKs.
        location /v1/identity {
            proxy_pass http://localhost:3000;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_set_header Host $host;
        }
    
        # Block all other routes explicitly. Only localhost can use these routes. If you want to open your
        # server up so that anyone can publish to it you should comment this section out.
        location / {
            allow 127.0.0.1;
            deny all;
        }
    }

This configuration by default blocks all connections other than `/v1/identity` and `/v1/database/<database-name>/subscribe` which only allows the most basic functionality. This will prevent all remote users from publishing to your SpacetimeDB instance.

Enable the configuration:

    sudo ln -s /etc/nginx/sites-available/spacetimedb /etc/nginx/sites-enabled/

Restart Nginx:

    sudo systemctl restart nginx

### Configure Firewall[​](#configure-firewall "Direct link to Configure Firewall")

Ensure your firewall allows HTTPS traffic:

    sudo ufw allow 'Nginx Full'
    sudo ufw reload

Step 4: Secure with Let's Encrypt[​](#step-4-secure-with-lets-encrypt "Direct link to Step 4: Secure with Let's Encrypt")
-------------------------------------------------------------------------------------------------------------------------

### Install Certbot[​](#install-certbot "Direct link to Install Certbot")

    sudo apt install certbot python3-certbot-nginx -y

### Obtain an SSL Certificate[​](#obtain-an-ssl-certificate "Direct link to Obtain an SSL Certificate")

Run this command to request a new SSL cert from Let's Encrypt. Remember to replace `example.com` with your own domain:

    sudo certbot --nginx -d example.com

Certbot will automatically configure SSL for Nginx. Restart Nginx to apply changes:

    sudo systemctl restart nginx

### Auto-Renew SSL Certificates[​](#auto-renew-ssl-certificates "Direct link to Auto-Renew SSL Certificates")

Certbot automatically installs a renewal timer. Verify that it is active:

    sudo systemctl status certbot.timer

Step 5: Verify Installation[​](#step-5-verify-installation "Direct link to Step 5: Verify Installation")
--------------------------------------------------------------------------------------------------------

On your local machine, add this new server to your CLI config. Make sure to replace `example.com` with your own domain:

    spacetime server add self-hosted --url https://example.com

If you have uncommented the `/v1/publish` restriction in Step 3 then you won't be able to publish to this instance unless you copy your module to the host first and then publish. We recommend something like this:

    spacetime build
    scp target/wasm32-unknown-unknown/release/spacetime_module.wasm ubuntu@<host>:/home/ubuntu/
    ssh ubuntu@<host> spacetime publish -s local --bin-path spacetime_module.wasm <database-name>

You could put the above commands into a shell script to make publishing to your server easier and faster. It's also possible to integrate a script like this into Github Actions to publish on some event (like a PR merging into master).

Step 6: Updating SpacetimeDB Version[​](#step-6-updating-spacetimedb-version "Direct link to Step 6: Updating SpacetimeDB Version")
-----------------------------------------------------------------------------------------------------------------------------------

To update SpacetimeDB to the latest version, first stop the service:

    sudo systemctl stop spacetimedb

Then upgrade SpacetimeDB:

    sudo -u spacetimedb -i -- spacetime --root-dir=/stdb version upgrade

To install a specific version, use:

    sudo -u spacetimedb -i -- spacetime --root-dir=/stdb install <version-number>

Finally, restart the service:

    sudo systemctl start spacetimedb

Step 7: Troubleshooting[​](#step-7-troubleshooting "Direct link to Step 7: Troubleshooting")
--------------------------------------------------------------------------------------------

### SpacetimeDB Service Fails to Start[​](#spacetimedb-service-fails-to-start "Direct link to SpacetimeDB Service Fails to Start")

Check the logs for errors:

    sudo journalctl -u spacetimedb --no-pager | tail -20

Verify that the `spacetimedb` user has the correct permissions:

    sudo ls -lah /stdb/spacetime

If needed, add the executable permission:

    sudo chmod +x /stdb/spacetime

### Let's Encrypt Certificate Renewal Issues[​](#lets-encrypt-certificate-renewal-issues "Direct link to Let's Encrypt Certificate Renewal Issues")

Manually renew the certificate and check for errors:

    sudo certbot renew --dry-run

### Nginx Fails to Start[​](#nginx-fails-to-start "Direct link to Nginx Fails to Start")

Test the configuration:

    sudo nginx -t

If errors are found, check the logs:

    sudo journalctl -u nginx --no-pager | tail -20

---

# Backups & long-term storage

## Persistence vs backups (what you actually need)

### Persistence (already built in)
SpacetimeDB persists to disk under its root directory (for this doc: `--root-dir=/stdb`). That means:
- Service restarts: data should still be there.
- Machine reboots: data should still be there.

So you typically do **not** need a separate persistence store like SQLite just to “keep data after restart.”

### Backups (still required for self-hosting)
Backups protect you from:
- VM deletion / instance termination
- Disk failure
- Filesystem corruption
- Operator error (bad upgrade, accidental delete, etc.)

If you self-host and you care about data durability, you need **off-host backups**.

## What to back up
Back up the SpacetimeDB root directory you run with (here: `/stdb`), ideally on a dedicated disk/volume.

Backing up the entire root dir keeps you insulated from internal layout changes between versions.

## AWS no-downtime backup runbook (EC2 + EBS + AWS Backup) — recommended

This is the simplest no-downtime durability setup on AWS:
- Run SpacetimeDB on **EC2**
- Store all SpacetimeDB state on a dedicated **EBS data volume** mounted at `/stdb`
- Use **AWS Backup** to take scheduled **EBS snapshots** (snapshots are stored in S3 behind the scenes)

### Sizing guidance (your case: 1 DB, ~100GB expected)
- **EBS data volume**: start at **200 GB gp3** mounted at `/stdb`
  - Rationale: headroom for growth, fragmentation/compaction, and operational slack.
- **IOPS/throughput**: gp3 defaults are usually fine to start; you can tune later without downtime.

### Important: consistency level
EBS snapshots are typically **crash-consistent**. In practice this is often acceptable (equivalent to a sudden power loss at the storage layer), but you should validate by periodically restoring and verifying the database.

If you need “application-consistent” backups, you generally need a database-native backup/export or a brief quiesce/stop hook.

### Step 0: Create and attach the EBS data volume
In the AWS Console:
1) Create a new **gp3** EBS volume (e.g. 200GB) in the **same AZ** as your EC2 instance.
2) Attach it to the EC2 instance.

### Step 1: Format + mount the EBS volume at `/stdb`
On the EC2 instance, identify the attached device:
```sh
lsblk
sudo fdisk -l
```

Assume the new volume is `/dev/nvme1n1` (common on many EC2 Nitro instances). If yours differs, substitute accordingly.

Format it:
```sh
sudo mkfs.ext4 -F /dev/nvme1n1
```

Mount it:
```sh
sudo mkdir -p /stdb
sudo mount /dev/nvme1n1 /stdb
df -h | grep /stdb
```

Persist the mount across reboots using UUID:
```sh
sudo blkid /dev/nvme1n1
sudo nano /etc/fstab
```

Add a line (replace `YOUR_UUID_HERE`):
```sh
UUID=YOUR_UUID_HERE  /stdb  ext4  defaults,nofail  0  2
```

Test:
```sh
sudo umount /stdb
sudo mount -a
df -h | grep /stdb
```

Ensure permissions match the service user:
```sh
sudo chown -R spacetimedb:spacetimedb /stdb
```

### Step 2: Ensure SpacetimeDB uses `/stdb`
Your systemd unit already does this:
- `ExecStart=/stdb/spacetime --root-dir=/stdb start ...`

This ensures all state is stored on the dedicated data volume.

### Step 3: Configure AWS Backup
In AWS Console:
1) Go to **AWS Backup**
2) Create a **Backup plan** (example starting point):
   - Hourly backups, retain 48 hours
   - Daily backups, retain 14 days
   - Monthly backups, retain 6–12 months
3) Assign resources:
   - Select the EBS volume mounted at `/stdb` (tagging it like `Name=spacetimedb-data` helps)
4) (Optional but recommended) Copy backups to another region/account for defense in depth

### Step 4: Regularly test restores
At least monthly:
- Create a new EBS volume from a snapshot
- Attach it to a staging EC2 instance
- Mount it at `/stdb`
- Start SpacetimeDB
- Run a known `spacetime sql ...` verification query

## Restore strategy (AWS snapshot restore)

High-level restore flow:
1) Create a new EBS volume from a snapshot
2) Attach it to a recovery EC2 instance
3) Mount it at `/stdb`
4) Ensure ownership:
```sh
sudo chown -R spacetimedb:spacetimedb /stdb
```
5) Start the service:
```sh
sudo systemctl start spacetimedb
```
6) Verify with a known query via `spacetime sql ...`

## Non-AWS (or “folder-to-S3”) warning
Copying `/stdb` (or `~/.local/share/spacetime`) to S3 while SpacetimeDB is actively writing can produce inconsistent backups.

If you are not using EBS snapshots, prefer:
- filesystem snapshots (btrfs/ZFS/LVM) and then archive/upload the snapshot, or
- a brief maintenance window to stop SpacetimeDB, archive, upload, start.

## Disaster recovery checklist
Minimum production-ish checklist:

- [ ] Backups stored off-host (EBS snapshots / AWS Backup / separate account/region if needed)
- [ ] Backup schedule documented (hourly/daily/weekly/monthly based on your RPO)
- [ ] Retention policy defined and enforced
- [ ] Restore tested regularly (at least monthly)
- [ ] Alerting/monitoring if backups fail
- [ ] Documented RPO/RTO targets:
  - RPO (data loss tolerance): e.g. 1 hour
  - RTO (time to restore): e.g. 30 minutes

