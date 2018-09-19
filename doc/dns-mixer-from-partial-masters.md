Mixing partial DNS zones
========================

>   *We introduce a building block for DNS that mixes DNS zone data from
>   multiple masters.  Each of these “partial masters” provides only a part of
>   the data that zones want to publish.  Access control limits the resource
>   data that an authenticated partial master may publish.  Zones output by the
>   DNS mixer integrate content from any number of partial masters.*

There are at least two applications for mixing DNS zone data from various
sources:

-   Domain owners often need to setup records in DNS if they want to add a
    service.  For external services, this is currently done mostly by hand, in
    practice leading to static configurations.  With a DNS mixer, the resource
    data can be dynamically provided by the external service itself, under
    constraints that avoid overtaking the entire zone.

-   Public User ENUM currently relies on users editing their zones.  With a DNS
    mixer, users only need to regulate which parties may provide what kind of
    ENUM applications, with occassional priority/weight settings to resolve
    competition, but leave it to service providers to deliver the actual `NAPTR`
    records.

Other applications may benefit, including Dynamic DNS, Multicast DNS and DNS
service in peer-to-peer networks, but we focus on these two applications in the
sequel.

Implementation
--------------

The implementation can be centered around a number of current-day tools:

-   [Knot DNS](https://www.knot-dns.cz) can be used as the output stage of the
    DNS mixer.  It offers a transactional commandline with resource data
    addition/removal/querying, and automation of SOA counter regime and DNSSEC
    signing.

-   An alternative output stage could be a hidden master supporting `NOTIFY`,
    `AXFR` and `IXFR` messages.  Note that an `IXFR` pretty much captures the
    idea of a transaction, whose commit initiates `NOTIFY` messages.

-   [ldns-zonediff](https://github.com/SURFnet/ldns-zonediff) can be used to
    easily infer changes between zone files, even with Knot DNS statement
    output.  There is also a utility
    [ldns-mergezone](https://github.com/SURFnet/ldns-mergezone), but that may
    be too specifically geared towards DNSSEC zone rollovers.

-   [Go DNS](https://github.com/miekg/dns) may be a good basis on which to
    implement the DNS mixer.

Bulk Data Management
--------------------

The interface between partial masters and the DNS mixer is meant for bulk data,
potentially handling many zones from each partial master.  To this end, the
partial master may publish under any root zone that it likes, even the root zone
`.` and the DNS mixer will constrain what is actually published.  As a
variation, each partial master may be setup with a “virtual root” which is a
sequence of DNS labels which the DNS mixer will strip from the owner names sent
by the partial master.

The partial master can follow any policy for zone cut-off boundaries.  It may
publish one large zone or any number of smaller ones.  The DNS mixer is
configured with these zones, and welcomes any `NOTIFY` that falls under the
partial master’s virtual root and will happily send an `IXFR` query for it.  For
very large zones, `AXFR` is less desirable.  Whatever is delivered that way will
be filtered through the authorisation rules for the partial master.

Even though access control is used to decide which records the DNS mixer will
forward to its output stage, there is no feedback about this to the partial
masters.  Data is simply taken in and subjected to access control rules that may
change without informing the partial master.

Note that `SOA` serial counters are kept for the relationship about zone apexes
with each partial master, but these records are not forwarded to the output
stage.  Instead, any such records are rejected, stored only to allow an `IXFR`
request in response to a `NOTIFY` message.  These `SOA` serial counters have no
meaning beyond this relationship between the partial master and the DNS mixer.

It is possible, even likely, that multiple DNS mixer nodes redundantly publish
the data from the partial masters.  This means that the partial master can have
multiple clients to send `NOTIFY` signals to.  Similarly, the partial master may
publish the same data over multiple addresses, though IPv4 addresses are
deprecated.

Note that one update to a partial master zone may influence multiple output
zones.  Care should be taken not to end in a deadlock as a result of this.  It
is however reasonable to (explicitly) assume that only the DNS mixer outputs to
its backend, to constrain the risk.  Other approaches include locking zones all
at the same time, and/or in a particular order, with back-off on failure to lock
a zone.

The case for Reference Counting
-------------------------------

The same output data (defined by owner name, resource record type and resource
record data) should not be supplied by more than one partial master.  Our model
however, includes cases where competing service providers send in data, and it
should not be beneficial to publish and retract data that is also published by
competitors, to make it flap in the output.

This means (sigh) that we need to count how many publications have been made of
the same output data.  We might actually hash the canonical output data for
storage simplicity.  On the bright side, we now have an interface over which we
can query whether a record is already in the output stage, so we need not ask
for its idea about that.  We can simply add it to the output when the use count
goes from 0 to 1, or remove it when it goes from 1 to 0.

We should be clear on any attempts by partial masters to repeatedly upload the
same combination, and what its impact is on reference counting and local
storage.  The reference counting regime may actually lead to simpler
implementations elsewhere, by supporting multisets (through lists).

Access Control
--------------

There are a few rules to manage in the DNS mixer:

-   Partial masters may only publish resource data that matches with the access
    control setup for that partial master.

-   Access control may match, overrule or constrain the individual data fields.

-   When access control changes, this may lead to a different split of what is
    published and what is rejected.

-   No partial master may forward resource data to the output stage if it is
    already published there.

-   No partial master may retract resource data from the output stage if other
    resource records still want it published there.

Access control lists are specific to a given partial master, which may of course
run on multiple IPv6 and even IPv4 addresses.

Access control consists of any number of lines, constraining things like:

-   The owner name that may be published.  Possibly allow `*` wildcards.

-   The zone for the given owner name.

-   We shall work with a fixed TTL setting.

-   We shall assume class `IN` alone.

-   A resource record type that may be published.  Allow `ANY` as wildcard.  Never
    publish a `SOA` from a partial master in an output zone.

-   For each resource record type, constraints to some or all fields.  This may
    impose a list of permitted values, but also ranges for numerical fields that
    are then modified by offsetting for minimum values and capping or rejection
    for maximum values.

The purpose of access control is to make a policy-based split into published and
rejected resource records; if another partial master already published the same
resource record data under the same owner name, according to the output stage,
then a reference count must be incremented.  When no rule on the ACL matches a
resource record, it will be rejected.

Before removing a resource record, it is first verified to be present in the
published or rejected resource records; only if it is in the former will the
removal be supported.  Before forwarding it to the output stage, a check is made
if a reference count can be decremented to reach 0.

When changes are made to access control, the published and rejected resource
records must be re-evaluated.  The result may move resource records between the
lists, with corresponding zone data changes forwarded to the output stage.  (But
it is still not permitted to publish data that is already published for another
partial master.)

In general, the published and rejected resource records are committed or aborted
together with the output stage transactions.

Secure Transfers
----------------

Partial masters and the DNS mixer have a one-to-one relation that will usually
be configured explicitly; because of that, and because of the simplicity of
configuration, it is reasonable to assume `TSIG` relations.

We shall rely on Kerberos as the mechanism for `TSIG` signing for most of our
setups.  This allows us to depend on realm crossover setups that may be revoked
at any time.  It also provides an identity for `NOTIFY` and `IXFR` data, thus
simplifying the task of mapping DNS data to an authenticated account and
providing an authenticated client identity as a basis for authorisation.

Other projects and future developments are bound to ask for more than Kerberos
as a `TSIG` mechanism, so when it is trivial to add, we should include it.

Data Storage
------------

Overal data for the DNS mixer:

-   An output stage, with externally controlled zone configuration but zone
    data other than `SOA` records controlled by the DNS mixer and `SOA`
    treated specially as defined below.

-   A key-value database for Reference Counting.

-   The TTL value to apply to all resource records in the output stage.

Per partial master:

-   Authenticating client identity

-   IP addresses for name servers

-   Virtual root, and to what this is internally rewritten.

Per partial master’s zone:

-   When known, the last accepted SOA with serial count, to enable `IXFR`
    instead of `AXFR`.

-   Access control list in terms of internal owner names.  For each, the
    number of labels to remove from the beginning to find the zone name
    receiving any changes in the output stage (only when fixed).

-   A set (or multiset) with published zone data.

-   A set (or multiset) with rejected zone data.

Per output zone:

-   Nothing, really.  The output stage handles that.

Procedures
----------

Updates are usually processed as `IXFR` changes, so they combine a number of
resource record removals and additions into one transaction for each impacted
zone.  An `AXFR` can be handled like an `IXFR` by first removing anything that is
currently on the published or rejected lists.

Elementary procedures are needed for:

-   Incrementing a Publication Reference Count

-   Decrementing a Publication Reference Count

-   Checking a Resource Record against an ACL

-   Adding a Resource Record

-   Removing a Resource Record

-   Adding an ACL Rule

-   Removing an ACL Rule

These procedures are detailed below, under the assumption that authentication
has taken place, but not authorisation.

### Incrementing a Publication Reference Count

Psuedocode for adding a resource record `RR`.  This conceals an optimisation with absent
elements when their value is 0:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
inc_pubrefcount (RR) {
    pubrefcount [hash (RR)] += 1;
    if pubrefcount [hash (RR)] == 1 then
        output_add_rr (RR);
    endif
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Decrementing a Publication Reference Count

Psuedocode for deleting a resource record `RR`.  This conceals an optimisation with absent
elements when their value is 0:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
dec_pubrefcount (RR) {
    if pubrefcount [hash (RR)] == 1 then
        output_del_rr (RR);
    endif
    pubrefcount [hash (RR)] -= 1;
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Checking a Resource Record against an ACL

Pseudocode for checking if the ACL for partial master `PM` accepts resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
acl_ok (PM,RR) {
    foreach AR in list_acl (PM) do
        if acl_match (AR,RR) then
            return true
        endif
    endfor
    return false
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Adding a Resource Record

Pseudocode acting for a partial master `PM` on a resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
add_rr (PM,RR) {
    if acl_ok (PM,RR) then
        append_published (PM,RR);
        inc_pubrefcount (RR);
    else
        append_rejected (PM,RR);
    endif
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Removing a Resource Record

Pseudocode acting for a partial master `PM` on a resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
del_rr (PM,RR) {
    if acl_ok (PM,RR) then
        remove_published (PM,RR);
        dec_pubrefcount (RR);
    else
        remove_rejected (PM,RR);
    endif
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Adding an ACL Rule

Pseudocode acting for a partial master `PM` on an ACL rule `AR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
add_acl_rule (PM,AR) {
    foreach RR in list_rejected (PM) do
        if acl_match (AR,RR) then
            remove_rejected (PM,RR);
            append_published (PM,RR);
            inc_pubrefcount (RR);
        endif
    endfor
    append_acl (PM,AR);
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Removing an ACL Rule

Pseudocode acting for a partial master `PM` on an ACL rule `AR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
del_acl_rule (PM,AR) {
    remove_acl (PM,AR);
    foreach RR in list_accepted (PM) {
        if acl_match (AR,RR) then
            drop := true;
            foreach AR2 in list_acl (PM) do
                if acl_match (AR2,RR) then
                    drop := false;
                endif
            endfor
            if drop then
                dec_pubrefcount (RR);
                remove_published (PM,RR);
                append_rejected (PM,RR);
            endif
        endif
    endfor
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

External Procedures
-------------------

There are a few constraints to using this service properly.

-   Zones must be known to the output stage as long as ACL rules may publish
    information about them.

-   Zones must not be locked by external processes, or must only be locked in
    the same fixed order as used by the DNS mixer.

-   Note that local zones too can be fed to the DNS mixer from an (internal)
    partial master.

There is also some need to facilitate external processes.

-   We should integrate a Pulley Backend to add and remove ACL rules.  This means
    that the API needed for ACL rule changes needs rules to add and remove
    tuples of zone, sublabels, and a formal text with further constraints to
    elements.  The API must support a transactional context for such ACL changes.

-   We need to add and remove output zones as well as partial masters and
    partial masters' zones.
