Mixing partial DNS zones
========================

>   *We introduce a building block for DNS that mixes DNS zone data from
>   multiple masters.  Each of these “partial masters” provides only a part of
>   the data that zones want to publish.  Access control limits the resource
>   data that an authenticated partial master may publish.  Zones output by the
>   DNS mixer integrate content from any number of partial masters.*

This document describes a new kind of software, described as "DNS mixers",
which basically does the following:

 1. Download zone data from any number of partial masters
 2. Pass this input data through (modding) ACLs to produce output zone data
 3. Supply the union of published zone data to a DNS output stage

The process is dynamic; zone data may be added or removed by partial masters,
as well as  the ACL that maps it to output data, can be modified and lead
to recomputation.  The description below aims to only process changes for such
adaptions.


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


The case for Multisets in the Output Stage
------------------------------------------

The same output data (defined by owner name, resource record type and resource
record data) should not be supplied by more than one partial master.  Our model
however, includes cases where competing service providers send in data, and it
should not be beneficial to publish and retract data that is also published by
competitors, to make it flap in the output.

This means (sigh) that we need to keep track of how many publications have
been made of the same output data.  We can do this by producing a multiset
of resource records in the output stage.  When producing output, multiples
will be combined into one entry.  We should be exceptionally careful about
`IXFR` computations, which may only add a resource record when its count
goes from 0 to >0 instances, and only remove it when its count drops from >0
back to 0 instances.

While producing a set of resource records as output from a zone's multiset,
differing TTL values for the same owner name and resource record type can
be reduced to the lowest value.  Note that this leads to some complexity
during the calculation of `IXFR` messages; an added resource record may
reduce the TTL just like the removal of a resource record may increase the
TTL.  This would cause the retraction of resource records on account of
their previous TTL, and their immediate republication with another TTL.
This complexity only unfolds during differential computing; the effort
of this efficiency disappears when simply computing the old and new zone
data and comparing them to produce an `IXFR` message.


Access Control and Resource Record Mapping
------------------------------------------

There are a few rules to manage in the DNS mixer:

-   Partial masters may only publish resource data that matches with the access
    control setup for that partial master.

-   Access control may match, overrule or constrain the individual data fields.

-   When access control changes, this may lead to different output zone data.

-   No partial master may forward resource data to the output stage if it is
    already published there.

-   No partial master may retract resource data from the output stage if other
    resource records still want it published there.

Access control lists are specific to a given partial master, which may of course
run on multiple IPv6 and even IPv4 addresses.

Access control consists of any number of lines, constraining things like:

-   The owner name that may be published.  Possibly allow `*` wildcards.

-   The zone for the given owner name.

-   TTL settings are bounded to sane extremes by default; these may be
    overridden explicitly.  When resource records arrive from different
    partial masters they may request different TTL values for the same
    owner name and resource record type; in such cases, the lowest value
    is used for all the resource records involved.

-   By default, we assume class `IN`, but there is a way to express class `CH`
    in ACL Rules.

-   A resource record type that may be published.  Allow a wildcard form.  Never
    publish a `SOA`, `IXFR` or `AXFR` from a partial master in an output zone.

-   For each resource record type, constraints to some or all fields.  This may
    impose a list of permitted values, but also ranges for numerical fields that
    are then modified by offsetting for minimum values and capping or rejection
    for maximum values.

Though access control imposes a constraint on what resource records may pass
from the input zone to one or more output zones, there is an added facility of
modifying the resource records while in transit.  It is even possible to
produce multiple resource records, perhaps in different zones, as a result of
ACL processing.  Individual ACL Rules would produce zero or one output
resource record, but the ACL as a whole produces a multiset of resource records,
possibly specifying a zone to which each may be assigned.  Note that the zone
specification can speed up the detection of the output zone, but it may also
be used to distinguish a parent and child zone where the zone apex is concerned.

Before removing a resource record, it is passed through the ACL once more, with
the intent to produce the same output, so it may be retracted.  Since the output
for each zone is maintained as a multiset of resource records, the removal of
one resource record from one partial master would not cause the removal of the
output resource data if it is still supported by another partial master.

When changes are made to access control for a partial master's zone, then all
resource records in that zone must be re-evaluated.  The removal of an ACL Rule
causes the removal of any produced records from their targeted output zones; the
addition of an ACL Rule causes appending any produced records to their targeted
output zones.

In general, changes to the input and output zones and the ACLs for partial master
zones are committed or aborted together with the output stage transactions.


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

Per partial master:

-   Authenticating client identity

-   IP addresses for name servers

-   Virtual root, and to what this is internally rewritten.

Per partial master’s zone:

-   When known, the last accepted SOA with serial count, to enable `IXFR`
    instead of `AXFR`.  Alternatively, move this over to the output zone.

-   Access control list in terms of internal owner names.  For each, the
    number of labels to remove from the beginning to find the zone name
    receiving any changes in the output stage (when known).

-   A multiset with resource records processed as input from the master zone.

Per output zone:

-   A multiset of resource records for each zone.

-   The last published SOA with serial count.

-   Historic information from which to infer `IXFR` data, or the resulting
    `IXFR` data itself.


Procedures
----------

Updates from partial masters are usually processed as `IXFR` changes, so they combine a number of
resource record removals and additions into one transaction for each impacted
zone.  An `AXFR` can be handled like an `IXFR` by first removing any resource record that was
hitherto in the partial master's zone.  As long as these removals and additions are all
done in one transaction, the result for the output zones should be consistent.

Output zones could be added automatically when ACL processing demands a specific
output zone.  This avoids dropping records that may later need to go into a
certain place.  We should think about any resource records that are not explicitly
assigned to a zone, and how they could be redistributed when a zone is added that
overlaps such resource records.  It would be helpful if ACL Rules always decide
on a zone to use, even if that is based on a simple default idea such as a TLD plus
one more level.  This would not be supportive of subdomains, except when they are
explicitly configured to produce a zone.

Elementary procedures are needed for:

-   Processing a Resource Record from a Partial Master

-   Adding a Resource Record

-   Removing a Resource Record

-   Adding an ACL Rule

-   Removing an ACL Rule

These procedures are detailed below, under the assumption that authentication
has taken place, but not authorisation.

### Processing a Resource Record from a Partial Master

Pseudocode for checking if the ACL for partial master `PM` accepts resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
acl_process (PM,RR) {
    OUT = list_empty ();
    foreach AR in list_acl (PM) do
        foreach <ZONE_OPT,NEWRR> in acl_rulemap (AR,RR) do
            list_append (OUT,<ZONE_OPT,NEWRR>);
        endfor
    endfor
    return OUT
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Adding a Resource Record

Pseudocode acting for a partial master `PM` on a resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
add_rr (PM,RR) {
    append_input (PM,RR);
    foreach <ZONE_OPT,NEWRR> in acl_process (PM,RR) do
        append_output (ZONE_OPT,NEWRR)
    endfor
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Removing a Resource Record

Pseudocode acting for a partial master `PM` on a resource record `RR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
del_rr (PM,RR) {
    foreach <ZONE_OPT,OLDRR> in acl_process (PM,RR) do
        remove_output (ZONE_OPT,OLDRR);
    done
    remove_input (PM,RR);
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Adding an ACL Rule

Pseudocode acting for a partial master `PM` on an ACL rule `AR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
add_acl_rule (PM,AR) {
    foreach RR in list_input (PM) do
        foreach <ZONE_OPT,NEWRR> in acl_rulemap (AR,RR) do
            append_output (ZONE_OPT,NEWRR);
        done
    done
    append_acl (PM,AR);
}
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

### Removing an ACL Rule

Pseudocode acting for a partial master `PM` on an ACL rule `AR`:

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
del_acl_rule (PM,AR) {
    remove_acl (PM,AR);
    foreach RR in list_input (PM) do
        foreach <ZONE_OPT,OLDRR> in acl_rulemap (AR,RR) do
            remove_output (ZONE_OPT,OLDRR);
        endfor
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
