# Access Control Rules

> *We specify access control on DNS data.  A simple textual language is
> perhaps the simplest; it may be interpreted in the DNS mixer, or
> first compiled to a binary intermediate format.  The general rule is
> that the DNS mixer does not need to know about the resource data format
> but it does need to treat some types in a special way.*

Each resource record type defines its own wire format, usually as a
sequence of simpler fields.  We can easily define an ACL syntax that
mentions the type and how to handle it.  The syntax describes a
regular language, meaning that it is fit for a lexical scanner, but
without nested structures there is no need for a stack-based parser.

An ACL Rule will approve of a resource record when it matches.  When no
ACL Rule matches, a resource record is rejected.  There is no notation
(yet) for ACL Rules that reject forms that may otherwise be accepted.
Changes to the ACL may recalculate these conditions without change to
the resource records.

An ACL Rule is a sequence of separate field matches, separated by
semicolons.  Each field consists of a field type name and an optional
set of constraining parameters.  Spaces and tabs are generously
permitted to improve readability.

Usually, a field is matched against a `name` and a `type` and any
resource data fields that may be desired.  It is a bad idea to
match resource data fields without having checked the `type` and
that may in fact be refused.  On the other hand, just matching the
`name` and not constraining the `type` or resource data would be
no problem, and has clearly defined semantics.

As an introductory example, an ACL Rule to allow `www.example.com`
with type `AAAA` and any address would look like

    name www.example.com. ; type AAAA

If the address range is to be constrained to unicast addresses, one
might extend the ACL Rule to

    name www.example.com. ; type AAAA ; u128 2000::&3000::

More elaborate examples will be shown below.

## Names

Field type `name` matches a DNS name.  This applies to the owner
name of a resource record in the first position, but also to the
argument of a `PTR` or `CNAME` record.

Add nothing behind the word to match any name,

    name

Add a fully qualified domain name ending in a dot to match
precisely that name,

    name www.example.com.

Add a name that is not fully qualified by not ending in a dot,
indicating a name underneath the context, using one of the two
equivalent forms,

    name www
    name www.@

Note that the context is the partial master's zone, not the output
zone.  This contextual zone is mentioned when the ACL Rule is added
or removed, and it will be the same in these two events.  When
names are not fully qualified, the implicit addition is `@` as in
the form above.  See below for implications this has on matching
and modifying zone names.

Names may start with a wildcard to match any concrete name supplied
by the resource record but not the wildcard itself,

    name *.people.example.com.
    name *.people.@

To also allow the wildcard in this position, explicitly specify it in
an extra Rule, using two asterisks in the place that will match the
wildcard and only the wildcard,

    name **.people.example.com.
    name **.people.@

The root zone `.` is defined with a special meaning in various
places, such as the `RP` and `SRV` record types.  To match it, use

    name .

However to match anything but this form, we need more.  We shall
write that with the form

    name *.

Now consider labels in the name to be numbered from 0 for the top-level,
incrementing by one for each label underneath.  Using these numbers, we
can filter and modify names by adding words after the name description
pattern.  Though it can be used on all name fields, the most important
impact is on the owner name, or the first field of an ACL Rule, where
it can be used to remove a partial monitor's virtual root and move it
to the actual zone name used in the output.

As a special note, when the `@` label is used in the name description,
it counts as a single label (because it is unfolded after the
application of the ACL Rule mapping and just before matching names
as part of the application of the ACL Rule).  The label may occur
in any position, not just at the top.  The notation `**` represents a
literal asterisk in the zone data, so it too counts as a single label.
This is not the case for the `*` label, which may cover multiple levels
of labels which are then separately counted.

We can filter out names with a given number of levels, with things like

    name *.com. 2
    name *.uk. 2-3
    name *.jp. 3-*

Note that the following forms are equivalent,

    name *.
    name *. 1-*

We can clip the name from the top at a given label, effectively removing
the given number of top-most labels, with

    name www.example.com.local. -1

We can set the number of labels involved in the output zone name, so as
to find the name of the targeted output zone,

    name www.example.com. =2
    name www.example.com.local. -1 =2

We can clip the name at the bottom label number to constrain the number
of levels that may pass to the given number plus one, with

    name *.people.example.com. ^3

This is a syntactical operation, so the following makes no sense:

    # Silly because ^3 passes the name with the wildcard:
    name **.people.example.com. ^3

We can add one or more labels on the top (or low-label-number) end,
perhaps after removing another top end, using a name prefixed with a dot,

    name *.example.com. -2 .example.org.
    name *.example.com. -2 .@

We can add a label on the bottom (or high-label) end, perhaps after removing another label,
for instance to change `www.example.com` into `my.example.com`,

    name www.example.com. ^2 +my

It might be argued that the same changes to names will be spread throughout the ACL.  This
suggests a more rigid model that may work, but the cost of finding the extra definition is
likely to be more than making the change here, especially because other fields may also be
modified.  The most probable modifier to distribute would be a simple `-3` to cut away the
three labels holding the sub-zone under which the connection with the DNS mixer's is held.

Some applications might call for multiple rewriting results.  It is better in these cases
to avoid ambiguity through filtering conditions, so that only one ACL Rule applies at any
time.  Future versions of the DNS mixer may report errors when a potential clash is found
between ACL Rules that are being loaded.  Better even, an indepenent analysis tool may be
used to support operators with a separate test, possibly integrated with zone monitoring.

Modified names are not stored in the lists of published or rejected resource records for
any of the partial masters' zones.  The mapping of names, and especially of owner names,
determines how the name server will publish the information.  Since updates to ACL Rules
will be reflected in updates to published zone data, this forms a nice mechanism to move
zone data around in a manner that introduces no ambiguity.  Especially note that removal
of an ACL Rule will reproduce a past name mapping in the same manner, and remove it too.


## Resource Record Types

The type of a resource record is a 16-bit unsigned number written in
a nice way.

To skip the type field, allowing anything but `SOA`, `ANY`, most
DNSSEC record types and perhaps a few other explicit
exceptions that should never pass, we write

    type

To require a certain resource record type, we add its name or the
equivalent number and write

    type MX
    type 15

It would be an error to specify `SOA` as a type, and `ANY` is not
specified in this place; in addition, a few others like `AXFR`
and `IXFR` may have to be banned.  It may be useful to specify
DNSSEC types however, for special circumstances such as the
migration of a zone between signers or between hosting providers.

Additional words can be added to the `type` field and would indicate
a mapping from one type to possibly another.  These mappings would
be implemented in a dynamically loadable library, taking in a
resource record and making callbacks to insert zero, one or more
resource records for further processing.  These functions would
be setup with prepared arguments based on the parameters given after
the `type` field type and its first argument.

For example, one might state that a `DNSKEY` record is to be mapped
to SHA-256 with a statement like

    type DNSKEY key2ds 2

An underlying loadable library would define a name `key2ds` that
processes accordingly.  Fixed parameters supplied are the numeric
resource record type plus strings in `argc` and `argv` style,
`"key2ds"` and `"2"`.  These parameters pass into a preparation
(or "compiler") function, which outputs an opaque pointer that
is subsequently passed into an actual runtime (or "workhorse")
function.  This runtime function is provided with the actual
resource record that is being mapped, together with a callback
function where it can deliver the resource record, without
knowing whether this would be added or removed by that function.
Any other fields that may be modified are updated before this
function is called.

Note that no restrictions on the algorithms and such need to be
made here; the remainder of the ACL Rule can be used to express
those.

The same function name may be used for more than one resource
record type, for example the same function as before might also
be applied for

    type CDNSKEY key2ds 2

While literal changes from child to parent form could be done
with

    type CDNSKEY child2parent
    type CDS     child2parent

These are only examples however; the actual work would be done
in the plugin mechanism.

**TODO:** Is the `type` field the best location for such
mapper functions?  We might also allow an optional filter
at the end, with `map` virtual fields.  A sequence of filters
might even be used for that purpose.

**TODO:** Instead of the plugin library mechanism, we might
also define an abstract class that can be specialised and
compiled in.  There won't be too many operations to support
anyway.

## Integers

Many fields in DNS are integers, and often simply 16 bits unsigned
represented on the wire in network byte order (or big endian).
These are written as `u16` but other forms, such as `u128` for 128-bit
unsigned IPv6 addresses are also allowed, or to give a list,

    u8
    u16
    u32
    u64
    u128

The type in each case defines the size of the field, so the match may
be partial without causing confusion.  So, to skip an integer field
without matching its value, just specify the field name,

    u16

To match a range of values including boundaries, specify those that
should be constrained,

    u16 6-13
    u16 6-*
    u16 *-13

A shorthand form is to match an exact value,

    u16 9

It is possible to specify more than one such entry as alternatives,

    u16 9 6-7 13-19 20 25 *-3

These values are all decimal, to match their most common patterns
of use.

Bit patterns can be masked and compared with a notation value`&`mask.
These two numbers are always in hexadecimal notation, their size is
rounded up to the largest value, the up from odd to even if needed,
and the result used to mask and compare a prefix of the field.
The values may be written a sequence of unsigned 16-bit integers in
hexadecimal notation separated by colons.  We can use a double
semicolon `::` in one position to fill up with zeroes, as is done
for IPv6 addresses.

One exception to the above is the notation of only `::` in a mask;
this does not express the value 0 as that would be useless; it
instead expresses the value with all-ones.  The use of `::` with
numeric values is meaningful, so it is treated as a filler with
zero bits.  In other words, the mask `::` is all-ones and the
masks `::0` and `0::` are all-zeroes; all three forms span the
entire bit range.

To match the top 3 bits of an IPv6-address, use

    u128 2000::&e000::

To exactly match the `localhost` address, use one of

    u128 0:0:0:0:0:0:0:1&ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff
    u128 ::1&ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff
    u128 0:0:0:0:0:0:0:1&::
    u128 ::1&::
    u128 1

To match a locally used prefix `2001:db8:1234::/48`, use one of

    u128 2001:db8:1234&ffff:ffff:ffff
    u128 2001:db8:1234:0&ffff:ffff:ffff:0
    u128 2001:db8:1234::&ffff:ffff:ffff::

We can add or subtract an offset, causing rejecting on overflow,
applicable to any following statements, using the respective forms

    u16 +3
    u16 -7

A variation is to cap off a field to be no less or no more or no
different than a given value in what follows, meaning that its
value will be set to the floor or ceiling or exact value
respectively by the forms

    u16 _3
    u16 ^87
    u16 =8080

As for these five modification forms preceding any matches,
it is worth noting that the modifications may loose information,
leading to less matches, but otherwise are bijections.  As a
result, there is always a possibility of pushing these modifiers
to the end.  (For now, we can require this from the user; in a
future version we may do it for them.)  As an example, the
combination `+3 9-12` is the same as `6-9 +3`, the sequence
`_3 6-*` is the same as `6-* _3` and `^87 66-99` is the same
as `^87 66-87` and `66-* ^87` forms.

Note that the various forms may be combined to list a number of
alternative choices in different forms, as in

    u16 e000&ff00 6-13 +7 13-20


## Byte sequences

We can recognise byte sequences with the same value`&`mask
notation as defined for integers.  The assumption is that
the number of bits is fixed.  There are a few special forms
for this.

The last field may capture anything remaining with

    tail

A field with an 8-bit length prefix may be described as

    len8
    len16

In these last cases, the match is against the value of the
given length, never including the length field.

Integer forms may be used on byte sequences, but they are of
variable length in this case.  The special form of a lone
`::` as the mask also translates to all-ones for bit fields.
The insertion of `::` with other data inserts variable-sized
zero filling for both mask and value fields.

We can make an exact match against a full byte sequence by
specifying a desired base64 format starting and ending with
a `@` character.

Since we can interpret byte sequences as strings, it is also
possible to specify regular expressions starting and ending
with a `/` slash.
Multiple regular expressions may be provided for one field,
of which at least one should match.

We can also match literal strings, starting and ending with
`"`.
Multiple strings may
be provided for one field, of which at least one should
match.

## No more data

We can specify only fields that we find interesting, leaving
off any trailing portion in the ACL Rule.

To explicitly deny any further data fields, we can state

    end

This may seem odd, but imagine a `TXT` field, with its
variable number of `len8` fields, for which we might want to
limit the resource data to two fields only, using

    l8 ; l8 ; end

This would match `TXT "hello" "world"` but not `TXT "one"
"microsoft" "way"` due to its use of three strings and not
`TXT "stranded on mercury"` due to its use of a single string.
Without the `end` added here, only the latter form would have
been rejected.


## Class

This field represents the class of a resource record, usually
`IN` for the common Internet naming scheme.  In fact, the only
other class in some use is `CHAOS` for name server identity.
These two names can be literally used as a field type name,
without further requirements,

    in
    chaos

Except for their parsing behaviour, these forms are equivalent
to, respectively,

    u16 1
    u16 3


## Time To Live

This field represents a time to live or TTL, which is equivalent to
a 32-bit unsigned integer `u32`, but written as

    ttl

The constraints and modifications to lower and higher values are
all supported.  When nothing is mentioned, the range is constrained
to reasonable default extremes of an hour and a week,

    ttl _3600 ^604800

When no constraint is applied to the TTL by not mentioning the `ttl`
field at all, then these same defaults are applied.

Different TTL values could arrive in the same resource record set,
as identified by their label, class and type.  This is not intended,
so a resolution is needed to implement the deprecation of this value
in Section 5.2 of RFC 2181.


## Resource Data Length

This field represents the number of bytes in the resource data
as it travels over the wire.  Although not usually needed, there may
be a use for matching or explicitly skipping it, which is easy
because it is equivalent to `u16`, but written as

    rdlen

The constraints to lower and higher values, as well as fixed values,
are all supported.  There is no constraint to restrict the default
case, but one might require things like

    rdlen 16
    rdlen 16-32

Changes to the value of this field are not supported.


## Complete Resource Records

Under the assumption that the grammar given here translates to some
underlying format, it is desirable to completely match resource
records; this is why forms were introduced for `ttl` and `rdlen`,
as well as the `in` and `chaos` classes, along with common defaults
and a translation to simple `u16` and `u32` matches.

The format of a resource record is as follows:

                                    1  1  1  1  1  1
      0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |                                               |
    /                                               /
    /                      NAME                     /
    |                                               |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |                      TYPE                     |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |                     CLASS                     |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |                      TTL                      |
    |                                               |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |                   RDLENGTH                    |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--|
    /                     RDATA                     /
    /                                               /
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+

Clearly, the full sequence for matching this form is `name`, `type`,
`in` or `chaos`, `ttl`, `rdlen` and specific resource data fields.
A compiler for the ACL Rule format can arrange simpler instructions,
mostly `name`, `u16` and `u32` forms, with desired behaviours.

Considering the introductory example,

    name www.example.com. ; type AAAA ; u128 2000::&3000::

This would make good use of the field type names to recognise the
absent fields for class, TTL and resource data length, and insert
their defaults.  Effectively, the translation would be for

    name www.example.com. ; type AAAA ; in ; ttl _3600 ^604800 ; rdlen ; u128 2000::&3000::

Now that all fields have been written out, the syntactical sugar
can be remove to arrive at

    name www.example.com. ; u16 28 ; u16 1 ; u32 _3600 ^604800 ; u16 ; u16 2000&3000

This simpler form should be easy to handle, or it may be further
reduced to a byte code that could be processed lightning fast by a
filter inside the DNS mixer.  The need to parse the DNS data would
be dramatically simplified, most notably because no knowledge of the
various resource data formats is required in such a filter.

The result of localising this knowledge in a syntax-processing phase
for ACL Rules is that the DNS mixer itself need not be aware of the
format of any of the Resource Records!  It only needs to handle the
field types described herein (and just add field types when they are
desired by new resource data definitions) 

**TODO:** Check that all current field types are indeed covered!

## Example: NSEC3 passthru

Only in special cases is it useful
to pass `NSEC3` elements, but they do provide a nice
example with many field types, so we will describe them nonetheless:

                         1 1 1 1 1 1 1 1 1 1 2 2 2 2 2 2 2 2 2 2 3 3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |   Hash Alg.   |     Flags     |          Iterations           |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |  Salt Length  |                     Salt                      /
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |  Hash Length  |             Next Hashed Owner Name            /
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /                         Type Bit Maps                         /
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Remember that we start with the name and resource record
type.  So our ACL Rule to pass this type without further
requirements would be

    name ; type NSEC3 ; u8 ; u8 ; u16 ; len8 ; len8 ; tail

We can further restrict the fields by adding recognisable
values behind their names.


## Example: SRV with offsetting

SRV records are defined more loosely, without a wire diagram, but
their names are interesting.  Also of some interest is that the
priority may be shifted up or down to accommodate a desire to place
it elsewhere.  We may also force a value for the weight if we want
to control load balancing between independent parties.

The fields of the resource data of an SRV record hold:

  * Priority as an `u16`
  * Weight as an `u16`
  * Port as an `u16`
  * Target as a `name`

Let's say that we will welcome an LDAP record at a priority in the
range from 10, topping it at 20, and that the weight will be set to
35.  We shall pass the port only if it is 389 only.  We may want to
reject records that explicitly state absense of service.  All this
can be done with

    name _ldap._tcp ; type SRV ; u16 +10 ^20 ; u16 =35 ; u16 389 ; name *.

A slave service provider may offer a priority 10, weight 0 server
that we shall setup as our load-balanced alternative with weight
50; and others with priority 99 and beyond, at varying weights,
that will be our fallback of last resort (at priority 30 and
beyond).  We can use these two ACL Rules to achieve our wishes:

    name _ldap._tcp ; type SRV ; u16 10-20    ; u16 0 =50 ; u16 389 ; name *.
    name _ldap._tcp ; type SRV ; u16 99-* -69 ; u16       ; u16 389 ; name *.

These two Rules may be setup for a completely different partial
master than the original single one.  This allows us to have a
primary server and two slaves from a dedicated (and possibly
cheap) backup provider.  All these entries will end up in a
context-defined zone.


