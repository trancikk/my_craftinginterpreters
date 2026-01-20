//
// Created by mypa0215 on 1/20/2026.
//

#include <stdio.h>
#include <stdlib.h>

struct ListNode {
    char *val;
    struct ListNode *next;
    struct ListNode *prev;
};

typedef struct ListNode ListNode;

ListNode *create_list_node(char *val) {
    ListNode *node = (ListNode *) malloc(sizeof(ListNode));
    node->val = val;
    node->next = NULL;
    node->prev = NULL;
    return node;
}

ListNode *insert_after(ListNode *head, char *val) {
    ListNode *node = (ListNode *) malloc(sizeof(ListNode));
    node->val = val;
    node->prev = head;

    if (head->next != NULL) {
        head->next->prev = node;
        node->next = head->next;
    } else {
        head->next = node;
        node->next = NULL;
    }

    return node;
}

void delete(ListNode *node) {
    if (node == NULL) {
        return;
    }
    if (node->prev != NULL && node->next != NULL) {
        node->prev->next = node->next;
        node->next->prev = node->prev;
    } else if (node->prev != NULL) {
        node->prev->next = NULL;
    } else if (node->next != NULL) {
        node->next->prev = NULL;
    }
    free(node);
}

ListNode *find(ListNode *head, char *val) {
    for (ListNode *tmp = head; tmp != NULL; tmp = tmp->next) {
        if (tmp->val == val) {
            return tmp;
        }
    }
    return NULL;
}


int main() {
    ListNode *node = create_list_node("head");
    ListNode *node1 = insert_after(node, "1");
    delete(node1);
    ListNode *node2 = insert_after(node, "2");
    ListNode *node3 = insert_after(node2, "3");
    ListNode *node4 = insert_after(node3, "4");

    const ListNode *find_node = find(node, "4");
    if (find_node != NULL) {
        printf("List node = %s", find_node->val);
    }
    return 0;
}
