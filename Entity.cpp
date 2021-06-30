#include "raylib.h"
#include "Entity.hpp"


Entity::Entity(Vector2 position1, int sizeX, int sizeY) {
    position = position1;
    rectangle.width = sizeX;
    rectangle.height = sizeY;
        color = RAYWHITE;
        updateRectangle();
    }

Entity::Entity(Vector2 position1, int sizeX, int sizeY, Color setColor) {
    position = position1;
    rectangle.width = sizeX;
    rectangle.height = sizeY;
    color = setColor;
    updateRectangle();
}

Entity::Entity(int positionX, int positionY, int sizeX, int sizeY) {
    position.x = positionX;
    position.y = positionY;
    rectangle.width = sizeX;
    rectangle.height = sizeY;
    color = RAYWHITE;
    updateRectangle();
}

Vector2 Entity::getPosition() {
    return position;
}

Vector2 Entity::getVelocity() {
    return position;
}

Rectangle Entity::getRectangle() {
    return rectangle;
}

Rectangle* Entity::getRectanglePtr() {
    return &rectangle;
}

void Entity::setColor(Color colorC) {
    color = colorC;
}

bool Entity::checkCollision(Rectangle entityRectangle) {
    return CheckCollisionRecs(rectangle, entityRectangle);
}

void Entity::update() {
    updatePosition();
    updateRectangle();
}

void Entity::draw() {
    DrawRectangleRec(rectangle, color);
}
  
void Entity::updateRectangle() {
    rectangle.x = position.x + (rectangle.width / 2);
    rectangle.y = position.y + (rectangle.height / 2);
}

void Entity::updatePosition() {
    position.x = velocity.x + position.x;
    position.y += velocity.y;
}

